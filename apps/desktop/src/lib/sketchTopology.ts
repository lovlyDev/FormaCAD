import type { SketchOperation } from "./sketchDocument";
export interface SketchLoop { id: string; pointIds: string[]; lineIds: string[] }
export function sketchLoops(operation: SketchOperation): SketchLoop[] | null {
  const profile = operation.lines.filter((line) => !line.construction);
  const next = new Map(profile.map((line) => [line.startPointId, line]));
  const ends = new Set(profile.map((line) => line.endPointId));
  const points = new Set(operation.points.map((point) => point.id));
  if (next.size !== profile.length || ends.size !== profile.length || profile.some((line) =>
    !points.has(line.startPointId) || !points.has(line.endPointId))) return null;
  const visited = new Set<string>();
  const loops: SketchLoop[] = [];
  for (const first of profile) {
    if (visited.has(first.id)) continue;
    const pointIds: string[] = [], lineIds: string[] = [];
    let cursor = first;
    while (true) {
      if (visited.has(cursor.id)) return null;
      visited.add(cursor.id); pointIds.push(cursor.startPointId); lineIds.push(cursor.id);
      if (cursor.endPointId === first.startPointId) break;
      const following = next.get(cursor.endPointId);
      if (!following) return null;
      cursor = following;
    }
    if (pointIds.length < 3 || loops.length >= 8) return null;
    loops.push({ id: first.id, pointIds, lineIds });
  }
  return loops.length ? loops : null;
}
