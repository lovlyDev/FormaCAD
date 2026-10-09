import type { TypedCadDocument } from "../../../lib/typedCadDocument";
import type { TopologyReference } from "./topologyReference";

/** This only checks the authored route. Final BREP resolution belongs to the kernel. */
export function matchesReferenceRoute(document: TypedCadDocument, sourceFeatureId: string, reference: TopologyReference): boolean {
  const features = new Map(document.features.map(feature => [feature.id, feature]));
  const reversedPath: string[] = [];
  const visited = new Set<string>();
  let cursor = sourceFeatureId;
  while (!visited.has(cursor)) {
    visited.add(cursor);
    const feature = features.get(cursor);
    if (!feature) return false;
    if (cursor === reference.ownerFeatureId) {
      const sketch = typeof feature.operation.sketchId === "string" ? features.get(feature.operation.sketchId) : null;
      const constructorMatches = sketch?.operation.type === "rectangle" ? reference.role.startsWith("box-") : sketch?.operation.type === "circle" && reference.role.startsWith("cylinder-");
      return !feature.suppressed && feature.operation.type === "extrude" && !!constructorMatches && !!sketch && !sketch.suppressed &&
        reversedPath.reverse().join("\0") === reference.occurrencePath.join("\0");
    }
    if (!["translate", "rotate", "mirror"].includes(feature.operation.type) || typeof feature.operation.bodyFeatureId !== "string") return false;
    if (!feature.suppressed) reversedPath.push(cursor);
    if (reversedPath.length > 64) return false;
    cursor = feature.operation.bodyFeatureId;
  }
  return false;
}
