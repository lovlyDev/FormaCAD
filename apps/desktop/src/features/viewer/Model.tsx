import { useThree } from "@react-three/fiber";
import { useBodyOutline } from "./useBodyOutline";
import { pickCadEdge } from "./pickCadEdge";
import { selectedEdgeStroke } from "./selectedEdgeStroke";
import { useEffect } from "react";
import * as THREE from "three";
import { useWorkspace } from "../../stores/workspace";
import type { RenderMode } from "./renderMode";
import { useTheme } from "../../lib/theme";
import { useBodyAppearance } from "./appearance/useBodyAppearance";
import {
  faceForTriangle,
  faceReference,
  faceOverlay,
  faceTriangleCounts,
  type FaceSelection,
} from "./faceSelection";
import { cadEdges, edgeOverlay, type EdgeSelection } from "./edgeSelection";
export function Model({
  object,
  mode,
  selected,
  selectedFace,
  selectedEdge,
  selectionMode,
  revisionId,
  onPoint,
  interactive = true,
}: {
  object: THREE.Group;
  mode: RenderMode;
  selected: string | null;
  selectedFace: FaceSelection | null;
  selectedEdge: EdgeSelection | null;
  selectionMode: "body" | "face" | "edge";
  revisionId: string | null;
  onPoint: (p: THREE.Vector3) => void;
  interactive?: boolean;
}) {
  const { camera, gl, size } = useThree();
  const select = useWorkspace((s) => s.setSelected);
  const selectFace = useWorkspace((s) => s.setSelectedFace);
  const selectEdge = useWorkspace((s) => s.setSelectedEdge);
  const light = useTheme() === "light";
  useBodyAppearance(object, mode, selectionMode === "body" || mode === "ghost" ? selected : null, light);
  useBodyOutline(object, mode, selected, light);
  useEffect(() => {
    if (!selectedFace || selectedFace.revisionId !== revisionId) return;
    const mesh = object.getObjectByName(selectedFace.bodyId);
    if (!(mesh instanceof THREE.Mesh)) return;
    const overlay = faceOverlay(mesh, selectedFace.faceOrdinal);
    if (!overlay) return;
    mesh.add(overlay);
    return () => {
      overlay.removeFromParent();
      overlay.geometry.dispose();
      (overlay.material as THREE.Material).dispose();
    };
  }, [object, revisionId, selectedFace]);
  useEffect(() => {
    if (!revisionId) return;
    const overlays: (THREE.LineSegments | ReturnType<typeof selectedEdgeStroke>)[] = [];
    object.traverse((node) => {
      if (!(node instanceof THREE.Mesh) || node.userData.formaSelectionOverlay)
        return;
      const edges = cadEdges(node);
      if (!edges) return;
      edges.forEach((edge, index) => {
        const ordinal = index + 1;
        const selected =
          selectedEdge?.revisionId === revisionId &&
          selectedEdge.bodyId === node.name &&
          selectedEdge.edgeOrdinal === ordinal;
        if (selectionMode !== "edge" && !selected) return;
        const overlay = edgeOverlay(node, edge, ordinal, selected);
        if (!overlay) return;
        if (selectionMode !== "edge") overlay.raycast = () => {};
        node.add(overlay);
        if (selected) {
          const stroke = selectedEdgeStroke(edge, size.width, size.height);
          node.add(stroke);
          overlays.push(stroke);
        }
        overlays.push(overlay);
      });
    });
    return () =>
      overlays.forEach((overlay) => {
        overlay.removeFromParent();
        overlay.geometry.dispose();
        (overlay.material as THREE.Material).dispose();
      });
  }, [object, revisionId, selectedEdge, selectionMode, size.width, size.height]);
  return (
    <primitive
      object={object}
      onClick={(e: {
        stopPropagation: () => void;
        nativeEvent: MouseEvent;
        object: THREE.Object3D;
        point: THREE.Vector3;
        faceIndex: number | null;
        intersections: { object: THREE.Object3D; point: THREE.Vector3 }[];
      }) => {
        e.stopPropagation();
        if (!interactive) return;
        if (selectionMode === "edge") {
          if (!revisionId) return;
          const rect = gl.domElement.getBoundingClientRect();
          const hit = pickCadEdge(object, camera, { x: e.nativeEvent.clientX - rect.left, y: e.nativeEvent.clientY - rect.top }, rect.width, rect.height);
          if (!hit) { selectEdge(null); return; }
          select(hit.bodyId);
          selectEdge({ bodyId: hit.bodyId, edgeOrdinal: hit.edgeOrdinal, revisionId, sceneToken: object.uuid,
            ...(hit.topologyRef ? { topologyRef: hit.topologyRef } : {}),
            ...(hit.semanticKey ? { semanticKey: hit.semanticKey } : {}) });
          onPoint(hit.point.clone());
          return;
        }
        if (e.object.userData.formaSelectionOverlay) return;
        if (!(e.object instanceof THREE.Mesh)) return;
        select(e.object.name);
        if (
          selectionMode === "face" &&
          revisionId &&
          e.object instanceof THREE.Mesh
        ) {
          const counts = faceTriangleCounts(e.object);
          const ordinal =
            counts && e.faceIndex !== null
              ? faceForTriangle(counts, e.faceIndex)
              : null;
          if (ordinal !== null) {
            const reference = faceReference(e.object, ordinal);
            selectFace({
              bodyId: e.object.name,
              faceOrdinal: ordinal,
              revisionId,
              sceneToken: object.uuid,
              ...(reference ? { topologyRef: reference } : {}),
            });
          }
        }
        onPoint(e.point.clone());
      }}
    />
  );
}
