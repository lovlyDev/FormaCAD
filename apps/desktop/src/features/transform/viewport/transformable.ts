import type * as THREE from "three";
import type { Project } from "../../../types";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";

/** Imported meshes and bodies missing from the revision document cannot be committed. */
export function transformableBodies(
  object: THREE.Object3D,
  project: Project,
  bodyIds: readonly string[],
): boolean {
  if (!object.userData.formaNativeCadPreview) return false;
  const program = project.revisions.find(
    (item) => item.id === project.currentRevision,
  )?.program;
  const document = program ? readTypedCadDocument(program) : null;
  if (!document) return false;
  const known = new Set(document.bodies.map((body) => body.id));
  return bodyIds.length > 0 && bodyIds.every((id) => known.has(id));
}
