import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";

/** Real isolated worker: every revision shown in the M2 harness is rebuilt by OCCT. */
export function viewportTransformWorker() {
  const candidates = [
    process.env.FORMA_E2E_WORKER,
    path.resolve("src-tauri/target/debug/forma-cad-worker.exe"),
  ].filter((item): item is string => !!item);
  const worker = candidates.find((item) => fs.existsSync(item));
  if (process.platform !== "win32" || !worker) return null;
  fs.mkdirSync(path.resolve("../../.local"), { recursive: true });
  const stage = fs.mkdtempSync(
    path.resolve("../../.local/e2e-viewport-transforms-"),
  );
  const executable = path.join(stage, "forma-cad-worker.exe");
  fs.copyFileSync(worker, executable);
  const env = {
    ...process.env,
    PATH: `${process.env.FORMA_OCCT_ROOT ? path.join(process.env.FORMA_OCCT_ROOT, "win64/vc14/bin") : path.resolve("../../.local/occt-build/win64/vc14/bin")};${path.resolve("src-tauri/resources/occt")};${process.env.PATH ?? ""}`,
  };
  let runs = 0;
  const builds = new Map<string, { document: string; step: Buffer }>();
  function sealedQuery(revisionId: string, bodyId: string, query: unknown,
    operation: "snap_query" | "measure_reference") {
      const built = builds.get(revisionId);
      if (!built) throw Error(`unknown revision ${revisionId}`);
      const cwd = path.join(stage, `${++runs}-snap-${revisionId}`);
      fs.mkdirSync(cwd);
      fs.writeFileSync(path.join(cwd, "document.json"), built.document);
      fs.writeFileSync(path.join(cwd, "source.step"), built.step);
      const sha = (bytes: Buffer | string) =>
        crypto.createHash("sha256").update(bytes).digest("hex");
      execFileSync(executable, [], {
        cwd,
        input: JSON.stringify({
          protocolVersion: 1,
          operation,
          requestId: crypto.randomUUID(),
          bodyId,
          documentSha256: sha(built.document),
          sourceSha256: sha(built.step),
          sourceSize: built.step.length,
          ...(operation === "snap_query" ? { snapQuery: query } : { measurementQuery: query }),
        }),
        windowsHide: true,
        timeout: 120000,
        env,
      });
      const result = JSON.parse(
        fs.readFileSync(path.join(cwd, "result.json"), "utf8"),
      );
      if (result.status !== "completed") throw Error(JSON.stringify(result));
      const output = operation === "snap_query" ? "snap.json" : "measurement.json";
      return JSON.parse(fs.readFileSync(path.join(cwd, output), "utf8"));
  }
  return {
    snap: (revisionId: string, bodyId: string, query: unknown) =>
      sealedQuery(revisionId, bodyId, query, "snap_query"),
    measure: (revisionId: string, bodyId: string, query: unknown) =>
      sealedQuery(revisionId, bodyId, query, "measure_reference"),
    build(document: unknown, revisionId: string) {
      const cwd = path.join(stage, `${++runs}-${revisionId}`);
      fs.mkdirSync(cwd);
      const doc = { ...(document as object), revisionId };
      execFileSync(executable, [], {
        cwd,
        input: JSON.stringify({
          protocolVersion: 1,
          requestId: revisionId,
          document: doc,
        }),
        windowsHide: true,
        timeout: 120000,
        env,
      });
      const result = JSON.parse(
        fs.readFileSync(path.join(cwd, "result.json"), "utf8"),
      );
      if (result.status !== "completed") throw Error(JSON.stringify(result));
      const files: Record<
        string,
        { data: string; size: number; sha256: string }
      > = {};
      for (const [ext, filename, type] of [
        ["glb", "preview.glb", "model/gltf-binary"],
        ["step", "model.step", "application/step"],
      ]) {
        const bytes = fs.readFileSync(path.join(cwd, filename));
        files[`${revisionId}.${ext}`] = {
          data: `data:${type};base64,${bytes.toString("base64")}`,
          size: bytes.length,
          sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
        };
      }
      builds.set(revisionId, {
        document: JSON.stringify(doc),
        step: fs.readFileSync(path.join(cwd, "model.step")),
      });
      return {
        files,
        boundsMm: result.boundsMm as number[],
        program: JSON.stringify(doc),
      };
    },
  };
}

const literal = (mm: number) => ({ kind: "literal", mm });
export const baseDocument = {
  schemaVersion: 2,
  revisionId: "base",
  parameters: [],
  features: [
    {
      id: "profile",
      name: "Profile",
      operation: { type: "rectangle", width: literal(40), depth: literal(20) },
    },
    {
      id: "pad",
      name: "Pad",
      operation: {
        type: "extrude",
        sketchId: "profile",
        distance: literal(10),
      },
    },
  ],
  bodies: [{ id: "body", name: "Body", sourceFeatureId: "pad" }],
};
