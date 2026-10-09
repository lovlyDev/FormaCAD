import type { CadDocumentDiff } from "../../../lib/cadDiff";
import { readTypedCadDocument, type TypedCadDocument } from "../../../lib/typedCadDocument";
import type { Project } from "../../../types";

/** Object key order is serialization detail; array order is CAD history/geometry. */
export function cadValuesEqual(left: unknown, right: unknown): boolean {
  if (Object.is(left, right)) return true;
  if (Array.isArray(left) && Array.isArray(right))
    return left.length === right.length && left.every((value, index) => cadValuesEqual(value, right[index]));
  if (!left || !right || typeof left !== "object" || typeof right !== "object" || Array.isArray(left) || Array.isArray(right)) return false;
  const a = left as Record<string, unknown>, b = right as Record<string, unknown>;
  const keys = Object.keys(a);
  return keys.length === Object.keys(b).length && keys.every(key => Object.hasOwn(b, key) && cadValuesEqual(a[key], b[key]));
}

function changes<T extends { id: string }>(before: T[], after: T[]) {
  const older = new Map(before.map((value, index) => [value.id, { value, index }]));
  const newer = new Map(after.map((value, index) => [value.id, { value, index }]));
  return [...new Set([...older.keys(), ...newer.keys()])].flatMap(id => {
    const a = older.get(id) ?? null, b = newer.get(id) ?? null;
    return a && b && a.index === b.index && cadValuesEqual(a.value, b.value) ? [] : [{ id, before: a, after: b }];
  });
}

export function candidateDiff(base: Project, source: string): CadDocumentDiff | null {
  const after = readTypedCadDocument(source);
  if (!after) return null;
  const program = base.revisions.find(revision => revision.id === base.currentRevision)?.program;
  const before: TypedCadDocument | null = base.currentRevision
    ? readTypedCadDocument(program ?? "")
    : { schemaVersion: 2, revisionId: "empty", parameters: [], features: [], bodies: [] };
  if (!before) return null;
  return {
    fromRevisionId: before.revisionId, toRevisionId: after.revisionId,
    parameters: changes(before.parameters, after.parameters),
    features: changes(before.features, after.features), bodies: changes(before.bodies, after.bodies),
  };
}
