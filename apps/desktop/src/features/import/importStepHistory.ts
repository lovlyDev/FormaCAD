import type { ProjectFile } from "../../types";
import type { TypedCadDocument } from "../../lib/typedCadDocument";
export function importStepHistory(file: ProjectFile | undefined, revisionId: string): string | null {
  if (!file || !/\.(step|stp)$/i.test(file.name) || !file.sha256 || !/^[a-f0-9]{64}$/.test(file.sha256)) return null;
  const document: TypedCadDocument = { schemaVersion: 2, revisionId, parameters: [], features: [
    { id: "imported_step", name: file.name, operation: { type: "importStep", assetId: `step_${file.sha256}`, sha256: file.sha256 } },
  ], bodies: [{ id: "imported_body", name: file.name, sourceFeatureId: "imported_step" }] };
  return JSON.stringify(document, null, 2);
}
export function hasImportedSource(document: TypedCadDocument | null, file: ProjectFile | undefined): boolean {
  return !!file?.sha256 && !!document?.features.some(feature => feature.operation.type === "importStep" && feature.operation.sha256 === file.sha256 && feature.operation.assetId === `step_${file.sha256}`);
}
