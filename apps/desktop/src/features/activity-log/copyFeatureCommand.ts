import type { TypedCadFeature } from "../../lib/typedCadDocument";

export function copyFeatureCommand(feature: TypedCadFeature): string {
  return JSON.stringify({ command: "add_feature", feature: { ...feature, id: crypto.randomUUID() } }, null, 2);
}
