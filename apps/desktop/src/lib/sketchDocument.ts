import { z } from "zod";
import { t } from "../i18n";

const point = z.looseObject({ id: z.string(), xMm: z.number(), yMm: z.number() });
const line = z.looseObject({ id: z.string(), startPointId: z.string(), endPointId: z.string(), construction: z.boolean().optional() });
const constraint = z.looseObject({
  id: z.string(), kind: z.enum(["fixed", "horizontal", "vertical", "length", "coincident"]),
  lineId: z.string().optional(), pointId: z.string().optional(),
  firstPointId: z.string().optional(), secondPointId: z.string().optional(),
  xMm: z.number().optional(), yMm: z.number().optional(),
  distance: z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("literal"), mm: z.number() }),
    z.object({ kind: z.literal("parameter"), parameterId: z.string() }),
  ]).optional(),
});
const bindingValue = z.object({kind:z.literal("parameter"),parameterId:z.string(),scale:z.number().finite(),offsetMm:z.number().finite()});
const binding = z.discriminatedUnion("target",[
 z.object({id:z.string(),target:z.literal("point"),pointId:z.string(),axis:z.enum(["x","y"]),value:bindingValue}),
 z.object({id:z.string(),target:z.literal("origin"),axis:z.enum(["x","y","z"]),value:bindingValue}),
]);
const sketch = z.looseObject({
  type: z.literal("sketch2d"),
  plane: z.enum(["xy", "xz", "yz"]),
  originMm: z.tuple([z.number(), z.number(), z.number()]),
  points: z.array(point),
  lines: z.array(line),
  constraints: z.array(constraint),
  bindings:z.array(binding).optional(),
});

export type SketchOperation = z.infer<typeof sketch>;

export function readSketchOperation(value: unknown): SketchOperation | null {
  const parsed = sketch.safeParse(value);
  return parsed.success && parsed.data.points.length >= 3 && parsed.data.points.length <= 32
    && parsed.data.lines.length <= 64
    && parsed.data.lines.filter((line) => !line.construction).length >= 3 ? parsed.data : null;
}

export function createStarterSketch(): string {
  return JSON.stringify({
    schemaVersion: 2,
    revisionId: "draft",
    parameters: [],
    features: [
      { id: "sketch_1", name: t("Sketch 1"), operation: {
        type: "sketch2d", plane: "xy", originMm: [0, 0, 0],
        points: [
          { id: "point_a", xMm: -20, yMm: -10 },
          { id: "point_b", xMm: 20, yMm: -10 },
          { id: "point_c", xMm: 20, yMm: 10 },
          { id: "point_d", xMm: -20, yMm: 10 },
        ],
        lines: [
          { id: "line_ab", startPointId: "point_a", endPointId: "point_b" },
          { id: "line_bc", startPointId: "point_b", endPointId: "point_c" },
          { id: "line_cd", startPointId: "point_c", endPointId: "point_d" },
          { id: "line_da", startPointId: "point_d", endPointId: "point_a" },
        ],
        constraints: [],
      } },
      { id: "pad_1", name: t("Extrusion 1"), operation: {
        type: "extrude", sketchId: "sketch_1", distance: { kind: "literal", mm: 10 },
      } },
    ],
    bodies: [{ id: "body_1", name: t("Body 1"), sourceFeatureId: "pad_1" }],
  }, null, 2);
}
