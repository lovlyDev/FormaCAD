import { useEffect, useRef, useState } from "react";
import { Mesh, type Group } from "three";
import { t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import { captureMeasurement } from "../../reference-measurements/measurementCapture";
import { measureReference } from "../../reference-measurements/measurementApi";
import { cadEdges, type EdgeSelection } from "../../viewer/edgeSelection";
import { faceReference, faceTriangleCounts, type FaceSelection } from "../../viewer/faceSelection";
import type { SelectionFilter } from "../../viewer/selectionFilter";

export function useSmartSelection(
  object: Group,
  interactive: boolean,
  setMode: (mode: SelectionFilter) => void,
) {
  const selectedFace = useWorkspace((state) => state.selectedFace);
  const selectedEdge = useWorkspace((state) => state.selectedEdge);
  const revision = useWorkspace((state) => state.project?.currentRevision);
  const projectId = useWorkspace((state) => state.project?.id);
  const [notice, setNotice] = useState<{ seed: FaceSelection | EdgeSelection; value: "busy" | "failed" | number } | null>(null);
  const activeSeed = selectedFace ?? selectedEdge;
  const status = notice?.seed === activeSeed ? notice.value : null;
  const epoch = useRef(0);
  useEffect(() => () => { epoch.current++; }, [object, interactive, projectId, revision]);

  function linked() {
    const state = useWorkspace.getState();
    const face = state.selectedFace;
    if (!interactive || !face || face.sceneToken !== object.uuid ||
        face.revisionId !== state.project?.currentRevision) return;
    const body = object.getObjectByName(face.bodyId);
    if (!(body instanceof Mesh) || !body.visible || body.userData.formaSelectionLocked) return;
    state.setSelected(face.bodyId);
    setMode("body");
    setNotice(null);
  }

  async function select(chain = false) {
    const state = useWorkspace.getState();
    const face = chain ? null : state.selectedFace;
    const edge = state.selectedEdge;
    const seed = face ?? edge;
    if (!interactive || !seed || seed.sceneToken !== object.uuid ||
        seed.revisionId !== state.project?.currentRevision) return;
    const mesh = object.getObjectByName(seed.bodyId);
    if (!(mesh instanceof Mesh) || !mesh.visible || mesh.userData.formaSelectionLocked) return;
    const ordinal = face?.faceOrdinal ?? edge?.edgeOrdinal;
    if (!ordinal) return;
    const capture = captureMeasurement({
      project: state.project, bodyId: seed.bodyId, selectionToken: object.uuid, interactive,
      query: { kind: "topologySelection", mode: chain ? "tangentChain" : face ? "similarFaces" : "similarEdges", seed: ordinal },
    });
    if (!capture) { setNotice({ seed, value: "failed" }); return; }
    const request = ++epoch.current;
    setNotice({ seed, value: "busy" });
    try {
      const { result } = await measureReference(capture);
      if (request !== epoch.current) return;
      const current = useWorkspace.getState();
      if ((face ? current.selectedFace : current.selectedEdge) !== seed ||
          current.project?.id !== capture.request.projectId ||
          current.project.currentRevision !== capture.request.expectedRevision ||
          !mesh.visible || mesh.userData.formaSelectionLocked) { setNotice(null); return; }
      if (result.kind !== "topologySelection") throw Error();
      if (face) {
        const counts = faceTriangleCounts(mesh);
        if (!counts || result.ordinals.some((index) => !counts[index - 1])) throw Error();
        current.setFaceSelection(result.ordinals.map((faceOrdinal) => ({
          bodyId: seed.bodyId, revisionId: seed.revisionId, sceneToken: object.uuid, faceOrdinal,
          topologyRef: faceReference(mesh, faceOrdinal) ?? undefined,
        })), face);
        setMode("face");
      } else if (edge) {
        const edges = cadEdges(mesh);
        if (!edges || result.ordinals.some((index) => !edges[index - 1]?.points.length)) throw Error();
        current.setEdgeSelection(result.ordinals.map((edgeOrdinal) => ({
          bodyId: seed.bodyId, revisionId: seed.revisionId, sceneToken: object.uuid, edgeOrdinal,
          topologyRef: edges[edgeOrdinal - 1].topologyRef,
          semanticKey: edges[edgeOrdinal - 1].semanticKey ?? undefined,
        })), edge);
        setMode("edge");
      }
      setNotice({ seed, value: result.ordinals.length });
    } catch {
      if (request === epoch.current) setNotice({ seed, value: "failed" });
    }
  }

  return {
    canLink: interactive && selectedFace?.sceneToken === object.uuid,
    canSelect: interactive && (selectedFace ?? selectedEdge)?.sceneToken === object.uuid,
    canChain: interactive && selectedEdge?.sceneToken === object.uuid,
    linked,
    similar: () => void select(),
    chain: () => void select(true),
    status: status === "busy" ? t("Selecting exact topology…")
      : status === "failed" ? t("Exact topology selection is unavailable.")
      : status === null ? null : t("Selected elements: {{value0}}", { value0: status }),
  };
}
