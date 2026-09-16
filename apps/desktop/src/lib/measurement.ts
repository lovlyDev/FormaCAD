import { t, fixedNumber } from "../i18n";
import { Vector3, MathUtils } from "three";
export type MeasureKind = "distance" | "angle" | "circle";
export function measurement(points: Vector3[], kind: MeasureKind): string {
  const required = kind === "distance" ? 2 : 3;
  if (points.length < required)
    return t("Выберите точку {{value0}} из {{value1}}", {
      value0: points.length + 1,
      value1: required,
    });
  const [a, b, c] = points;
  if (kind === "distance")
    return t("{{value0}} mm", { value0: fixedNumber(a.distanceTo(b)) });
  const u = a.clone().sub(b),
    v = c.clone().sub(b);
  if (u.lengthSq() < 1e-12 || v.lengthSq() < 1e-12)
    return t("Выберите разные точки");
  if (kind === "angle")
    return `${fixedNumber(MathUtils.radToDeg(u.angleTo(v)))}°`;
  const cross = u.clone().cross(v).length();
  if (cross < 1e-8 * u.length() * v.length())
    return t("Точки лежат на одной прямой");
  const radius =
    (a.distanceTo(b) * b.distanceTo(c) * c.distanceTo(a)) / (2 * cross);
  return t("R {{value0}} mm · Ø {{value1}} mm", {
    value0: fixedNumber(radius),
    value1: fixedNumber(radius * 2),
  });
}
