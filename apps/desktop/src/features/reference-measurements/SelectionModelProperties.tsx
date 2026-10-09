import type { ComponentProps } from "react";
import { ModelInspector } from "../../app/ModelInspector";
import { ExactReferenceMeasurement } from "./ExactReferenceMeasurement";
import { PairMeasurement } from "../pair-measurements/PairMeasurement";

type Props = ComponentProps<typeof ModelInspector> & {
  selectedBodyId: string | null;
  interactive: boolean;
  hostAvailable: boolean;
  sceneToken?: string;
};

/** Whole-scene properties and authored selection measurements retain separate scopes. */
export function SelectionModelProperties({ selectedBodyId, interactive, hostAvailable, sceneToken = "", ...inspector }: Props) {
  const bodyId = inspector.selectedEdge?.bodyId ?? inspector.selectedFace?.bodyId ?? selectedBodyId;
  return <div className="selection-model-properties">
    <ModelInspector {...inspector} />
    <ExactReferenceMeasurement project={inspector.project} bodyId={bodyId}
      edge={inspector.selectedEdge} face={inspector.selectedFace}
      interactive={interactive} hostAvailable={hostAvailable} />
    <PairMeasurement bodyId={bodyId} sceneToken={sceneToken} interactive={interactive} hostAvailable={hostAvailable} />
  </div>;
}
