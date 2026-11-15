import { expect, it } from "vitest";
import { igesReportSchema } from "./igesSchema";
function report() {
  const metrics = { volumeMm3: 8000, areaMm2: 2800, solidCount: 1, faceCount: 6, edgeCount: 12, boundsMm: [40, 20, 10] };
  return { schemaVersion: 1, requestId: crypto.randomUUID(), direction: "import", sourceSha256: "a".repeat(64), sourceSize: 100,
    source: metrics, roundTrip: { ...metrics }, outputSha256: "b".repeat(64), outputSize: 100,
    previewSha256: "c".repeat(64), previewSize: 100 };
}
it("rejects geometry loss, invalid measurements, wrong preview scope and forged extra fields", () => {
  const valid = report(); expect(igesReportSchema.safeParse(valid).success).toBe(true);
  for (const roundTrip of [{ ...valid.roundTrip, volumeMm3: 7999 }, { ...valid.roundTrip, solidCount: 2 },
    { ...valid.roundTrip, boundsMm: [NaN, 20, 10] }, { ...valid.roundTrip, areaMm2: 0 }])
    expect(igesReportSchema.safeParse({ ...valid, roundTrip }).success).toBe(false);
  expect(igesReportSchema.safeParse({ ...valid, direction: "export" }).success).toBe(false);
  expect(igesReportSchema.safeParse({ ...valid, previewSha256: null }).success).toBe(false);
  expect(igesReportSchema.safeParse({ ...valid, outputPath: "C:/somewhere" }).success).toBe(false);
});
