export type TransformQuantity = "length" | "angle" | "scale";
export type NumericInput =
  | { status: "empty" | "incomplete" }
  | { status: "invalid"; reason: "syntax" | "unit" | "range" | "expression" }
  | { status: "valid"; value: number };
const units: Record<TransformQuantity, Record<string, number>> = {
  length: {
    "": 1,
    mm: 1,
    мм: 1,
    cm: 10,
    см: 10,
    m: 1000,
    м: 1000,
    in: 25.4,
    '"': 25.4,
  },
  angle: { "": 1, deg: 1, "°": 1, rad: 180 / Math.PI },
  scale: { "": 1, "%": 0.01 },
};
/** Fixed grammar; quantities are converted to mm, degrees or dimensionless factors. Never eval. */
export function parseTransformNumber(
  input: string,
  quantity: TransformQuantity,
): NumericInput {
  const text = input.trim().toLocaleLowerCase();
  if (!text) return { status: "empty" };
  if (text.length > 48) return { status: "invalid", reason: "range" };
  if (text.startsWith("=")) return { status: "invalid", reason: "expression" };
  if (/^[+-]?(?:[.,])?$/.test(text)) return { status: "incomplete" };
  const match = /^([+-]?(?:\d+(?:[.,]\d*)?|[.,]\d+))\s*([a-zа-яё°%"]*)$/u.exec(
    text,
  );
  if (!match) return { status: "invalid", reason: "syntax" };
  const multiplier = units[quantity][match[2]];
  if (multiplier === undefined) return { status: "invalid", reason: "unit" };
  const value = Number(match[1].replace(",", ".")) * multiplier;
  if (
    !Number.isFinite(value) ||
    (quantity === "length" && Math.abs(value) > 10000) ||
    (quantity === "angle" && Math.abs(value) > 360) ||
    (quantity === "scale" && (value < 1e-6 || value > 1000))
  )
    return { status: "invalid", reason: "range" };
  return { status: "valid", value };
}
