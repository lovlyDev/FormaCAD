import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import * as THREE from "three";
import { loadModel } from "../src/lib/files";
import { applyTheme, useTheme } from "../src/lib/theme";
import { useWorkspace, newProject } from "../src/stores/workspace";
import { Model } from "../src/features/viewer/Model";
import { cadEdges } from "../src/features/viewer/edgeSelection";
import { pickCadEdge } from "../src/features/viewer/pickCadEdge";
import { EdgeFilletEditor } from "../src/features/model-editor/EdgeFilletEditor";
import { Modal } from "../src/components/ui";
import { t } from "../src/i18n";
import { disposeModel } from "../src/lib/model";
import "../src/styles.css";
import "../src/app/dialogs/ProgramDialog.css";
import "../src/features/model-editor/EditorDialog.css";

function Probe({ object }: { object: THREE.Group }) {
  const { camera, gl, size } = useThree();
  useEffect(() => { camera.lookAt(new THREE.Box3().setFromObject(object).getCenter(new THREE.Vector3())); camera.updateMatrixWorld(); }, [camera, object]);
  useFrame(() => {
    object.updateMatrixWorld(true);
    const rect = gl.domElement.getBoundingClientRect();
    const candidates: unknown[] = [], catalogs: Record<string, unknown> = {};
    object.traverse(node => {
      if (!(node instanceof THREE.Mesh) || node.userData.formaSelectionOverlay) return;
      const edges = cadEdges(node) ?? [];
      catalogs[node.name] = edges.map(edge => ({ topologyRef: edge.topologyRef ?? null, semanticKey: edge.semanticKey }));
      edges.forEach((edge, index) => {
        if (edge.points.length < 2) return;
        const a = new THREE.Vector3(...edge.points[0]).applyMatrix4(node.matrixWorld);
        const b = new THREE.Vector3(...edge.points[1]).applyMatrix4(node.matrixWorld);
        const point = a.lerp(b, .5).project(camera), x = (point.x + 1) * size.width / 2, y = (1 - point.y) * size.height / 2;
        const hit = pickCadEdge(object, camera, { x, y }, size.width, size.height);
        if (hit?.bodyId === node.name && hit.edgeOrdinal === index + 1) candidates.push({ bodyId: node.name, x: rect.x + x, y: rect.y + y, reference: edge.topologyRef ?? null, semanticKey: edge.semanticKey });
      });
    });
    Object.assign(window, { __topologyProbe: { candidates, catalogs } });
  });
  return null;
}
function Harness({ initialObject, initialSource }: { initialObject: THREE.Group; initialSource: string }) {
  const [object, setObject] = useState(initialObject), [source, setSource] = useState(initialSource), [draft, setDraft] = useState(initialSource), [open, setOpen] = useState(false);
  const selected = useWorkspace(state => state.selected), edge = useWorkspace(state => state.selectedEdge);
  const revision = useWorkspace(state => state.project?.currentRevision ?? null);
  const light = useTheme() === "light";
  useEffect(() => {
    Object.assign(window, { __topologyDraft: draft, __topologySource: source, __topologySelection: edge, __topologyLoad: async (data: { source: string; glb: string; revision: string }) => {
      const next = await loadModel({ name: "preview.glb", kind: "model", size: 100, data: data.glb });
      setObject(next); setSource(data.source); setDraft(data.source); setOpen(false);
      const state = useWorkspace.getState();
      state.setSelectedEdge(null); state.setSelected(null);
      state.setProject({ ...state.project!, currentRevision: data.revision });
    } });
  }, [source, draft, edge]);
  useEffect(() => () => disposeModel(object), [object]);
  return <div data-testid="topology-harness" style={{ height: 760 }}>
    <button data-testid="open-editor" onClick={() => setOpen(true)}>{t("Edit model parameters")}</button>
    <Canvas camera={{ position: [120, 95, 130], fov: 38, near: .1, far: 1000 }} dpr={1}>
      <color attach="background" args={[light ? "#eef0f3" : "#1d2023"]} /><ambientLight intensity={1} /><directionalLight position={[50, 80, 50]} intensity={2} />
      <Probe object={object} /><Model object={object} mode="edges" selected={selected} selectedFace={null} selectedEdge={edge} selectionMode="edge" revisionId={revision} onPoint={() => {}} />
    </Canvas>
    <Modal open={open} onClose={() => setOpen(false)} title={t("Edit model parameters")} description={t("Preview the fillet, then build to save a new revision.")} wide className="model-editor-modal">
      <div className="model-editor-scroll"><EdgeFilletEditor source={draft} savedSource={source} disabled={false} onChange={setDraft} /></div>
    </Modal>
  </div>;
}
export async function mountTopologyHarness({ source, glb, revision }: { source: string; glb: string; revision: string }) {
  const project = newProject("Topology QA", "blank", "mm", "codex"); project.currentRevision = revision;
  useWorkspace.getState().setProject(project); useWorkspace.getState().setSelectedEdge(null);
  const object = await loadModel({ name: "preview.glb", kind: "model", size: 100, data: glb });
  Object.assign(window, { __topologyTheme: applyTheme }); applyTheme("dark");
  createRoot(document.getElementById("root")!).render(<Harness initialObject={object} initialSource={source} />);
}
