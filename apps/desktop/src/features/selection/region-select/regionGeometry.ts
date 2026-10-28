export type Point2 = readonly [number, number];
export type ScreenPrimitive =
  | readonly [Point2]
  | readonly [Point2, Point2]
  | readonly [Point2, Point2, Point2];
export type SelectionRegion =
  | { kind: "circle"; center: Point2; radius: number }
  | { kind: "polygon"; points: readonly Point2[] };
const cross = (a: Point2, b: Point2, c: Point2) =>
  (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
const finitePoint = (point: Point2) =>
  point.length === 2 && point.every(Number.isFinite);
function onSegment(point: Point2, a: Point2, b: Point2): boolean {
  return (
    Math.abs(cross(a, b, point)) <= 1e-7 &&
    point[0] >= Math.min(a[0], b[0]) - 1e-7 &&
    point[0] <= Math.max(a[0], b[0]) + 1e-7 &&
    point[1] >= Math.min(a[1], b[1]) - 1e-7 &&
    point[1] <= Math.max(a[1], b[1]) + 1e-7
  );
}
function edges(points: readonly Point2[]): [Point2, Point2][] {
  if (points.length < 2) return [];
  if (points.length === 2) return [[points[0], points[1]]];
  return points.map((point, index) => [
    point,
    points[(index + 1) % points.length],
  ]);
}
export function insidePolygon(
  point: Point2,
  polygon: readonly Point2[],
): boolean {
  let inside = false;
  for (let index = 0; index < polygon.length; index++) {
    const a = polygon[index],
      b = polygon[(index + 1) % polygon.length];
    if (onSegment(point, a, b)) return true;
    if (
      a[1] > point[1] !== b[1] > point[1] &&
      point[0] < ((b[0] - a[0]) * (point[1] - a[1])) / (b[1] - a[1]) + a[0]
    )
      inside = !inside;
  }
  return inside;
}
function segmentsIntersect(
  a: Point2,
  b: Point2,
  c: Point2,
  d: Point2,
): boolean {
  const abC = cross(a, b, c),
    abD = cross(a, b, d),
    cdA = cross(c, d, a),
    cdB = cross(c, d, b);
  return (
    (abC * abD < 0 && cdA * cdB < 0) ||
    onSegment(a, c, d) ||
    onSegment(b, c, d) ||
    onSegment(c, a, b) ||
    onSegment(d, a, b)
  );
}
function distanceSq(point: Point2, a: Point2, b: Point2): number {
  const dx = b[0] - a[0],
    dy = b[1] - a[1],
    lengthSq = dx * dx + dy * dy;
  const t = lengthSq
    ? Math.max(
        0,
        Math.min(
          1,
          ((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / lengthSq,
        ),
      )
    : 0;
  return (point[0] - a[0] - t * dx) ** 2 + (point[1] - a[1] - t * dy) ** 2;
}
export function boxRegion(start: Point2, end: Point2): SelectionRegion {
  return {
    kind: "polygon",
    points: [
      [start[0], start[1]],
      [end[0], start[1]],
      [end[0], end[1]],
      [start[0], end[1]],
    ],
  };
}
/** Validate and prepare lasso bounds/segments once per pointer sample, not per triangle. */
export function prepareRegion(
  region: SelectionRegion,
): (primitive: ScreenPrimitive) => boolean {
  if (region.kind === "circle") {
    if (
      !finitePoint(region.center) ||
      !Number.isFinite(region.radius) ||
      region.radius <= 0
    )
      return () => false;
    const r2 = region.radius ** 2;
    const center: Point2 = [...region.center];
    return (primitive) =>
      primitive.every(finitePoint) &&
      (primitive.some((point) => distanceSq(center, point, point) <= r2) ||
        edges(primitive).some(([a, b]) => distanceSq(center, a, b) <= r2) ||
        (primitive.length === 3 && insidePolygon(center, primitive)));
  }
  if (
    region.points.length < 3 ||
    region.points.length > 4096 ||
    !region.points.every(finitePoint)
  )
    return () => false;
  const points: Point2[] = region.points.map((point) => [...point]);
  const segments = edges(points);
  const xs = region.points.map((point) => point[0]),
    ys = region.points.map((point) => point[1]);
  const minX = Math.min(...xs),
    maxX = Math.max(...xs),
    minY = Math.min(...ys),
    maxY = Math.max(...ys);
  return (primitive) => {
    if (
      !primitive.every(finitePoint) ||
      primitive.every((point) => point[0] < minX) ||
      primitive.every((point) => point[0] > maxX) ||
      primitive.every((point) => point[1] < minY) ||
      primitive.every((point) => point[1] > maxY)
    )
      return false;
    return (
      primitive.some((point) => insidePolygon(point, points)) ||
      (primitive.length === 3 &&
        points.some((point) => insidePolygon(point, primitive))) ||
      edges(primitive).some(([a, b]) =>
        segments.some(([c, d]) => segmentsIntersect(a, b, c, d)),
      )
    );
  };
}
/** Screen intersection including a region fully contained inside a large face. */
export function regionIntersects(
  region: SelectionRegion,
  primitive: ScreenPrimitive,
): boolean {
  return prepareRegion(region)(primitive);
}
