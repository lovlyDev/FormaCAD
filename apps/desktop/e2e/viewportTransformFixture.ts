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
  return {
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
