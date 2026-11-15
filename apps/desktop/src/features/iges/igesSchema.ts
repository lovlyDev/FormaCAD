import { z } from "zod";
const digest = z.string().regex(/^[a-f0-9]{64}$/);
const size = z.number().int().positive().max(40 * 1024 * 1024);
const positive = z.number().finite().positive();
const count = z.number().int().positive();
export const igesMetricsSchema = z.strictObject({ volumeMm3: positive, areaMm2: positive,
  solidCount: count, faceCount: count, edgeCount: count,
  boundsMm: z.tuple([positive.max(1e9), positive.max(1e9), positive.max(1e9)]),
});
export const igesReportSchema = z.strictObject({ schemaVersion: z.literal(1), requestId: z.string().uuid(),
  direction: z.enum(["import", "export"]), sourceSha256: digest, sourceSize: size,
  source: igesMetricsSchema, roundTrip: igesMetricsSchema, outputSha256: digest, outputSize: size,
  previewSha256: digest.nullable(), previewSize: size.nullable(),
}).refine(report => {
  const close = (a: number, b: number) => Math.abs(a - b) <= Math.max(1e-4, Math.max(Math.abs(a), Math.abs(b)) * 1e-7);
  return report.source.solidCount === report.roundTrip.solidCount && close(report.source.volumeMm3, report.roundTrip.volumeMm3)
    && close(report.source.areaMm2, report.roundTrip.areaMm2) && report.source.boundsMm.every((v, i) => close(v, report.roundTrip.boundsMm[i]))
    && (report.direction === "import" ? report.previewSha256 !== null && report.previewSize !== null
      : report.previewSha256 === null && report.previewSize === null);
});
export type IgesReport = z.infer<typeof igesReportSchema>;
export type IgesMetrics = z.infer<typeof igesMetricsSchema>;
