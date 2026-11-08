import { createHash, randomUUID } from "node:crypto";
import type { viewportTransformWorker } from "./viewportTransformFixture";

export function buildPreviewPacket(
  worker: NonNullable<ReturnType<typeof viewportTransformWorker>>,
  source: string,
) {
  const id = randomUUID();
  const model = worker.build(JSON.parse(source), id);
  const { result } = worker.measure(id, "body", { kind: "bodyMetrics" });
  const { kind: _kind, extentsMm, ...metrics } = result;
  const metadata = Buffer.from(JSON.stringify({
    protocolVersion: 1, sourceSha256: createHash("sha256").update(source).digest("hex"),
    metrics: { ...metrics, boundsMm: extentsMm },
  }));
  const header = Buffer.alloc(4);
  header.writeUInt32LE(metadata.length);
  const glb = Buffer.from(model.files[`${id}.glb`].data.split(",")[1], "base64");
  return [...Buffer.concat([header, metadata, glb])];
}
