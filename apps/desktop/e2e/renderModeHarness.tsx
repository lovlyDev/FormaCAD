import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import * as THREE from "three";
import { loadModel, exportMesh } from "../src/lib/files";
import { applyTheme, useTheme } from "../src/lib/theme";
import { useWorkspace } from "../src/stores/workspace";
import { Select } from "../src/components/ui";
import { t } from "../src/i18n";
import { Model } from "../src/features/viewer/Model";
import { SectionScene } from "../src/features/viewer/sections/SectionScene";
import type { RenderMode } from "../src/features/viewer/renderMode";
import "../src/styles.css";

function Probe({ object }: { object: THREE.Group }) {
  const { camera, gl, scene } = useThree();
  const originals = useMemo(() => {
    const result = new Map<THREE.Mesh, THREE.Material | THREE.Material[]>();
    object.traverse(node => { if (node instanceof THREE.Mesh) result.set(node, node.material); });
    return result;
  }, [object]);
  useEffect(() => {
    camera.lookAt(new THREE.Box3().setFromObject(object).getCenter(new THREE.Vector3()));
    camera.updateMatrixWorld();
  }, [camera, object]);
  useFrame(() => {
    object.updateMatrixWorld(true);
    const meshes: Record<string, unknown> = {};
    originals.forEach((source, mesh) => {
      const material = mesh.material as THREE.MeshStandardMaterial;
      const outlines = mesh.children.filter(node => node.userData.formaAppearanceOutline) as THREE.LineSegments[];
      meshes[mesh.name] = { opacity: material.opacity, depthWrite: material.depthWrite, sourceRestored: mesh.material === source,
        clipping: material.clippingPlanes?.length ?? 0, color: material.color.getHexString(),
        outlines: outlines.map(line => ({ depthTest: (line.material as THREE.Material).depthTest, opacity: (line.material as THREE.Material).opacity })),
        sourceOpacity: (Array.isArray(source) ? source[0] : source).opacity };
    });
    const inner = object.getObjectByName("inner")!;
    const target = new THREE.Box3().setFromObject(inner).getCenter(new THREE.Vector3());
    const ray = new THREE.Raycaster(camera.position, target.clone().sub(camera.position).normalize());
    Object.assign(window, { __appearanceInspection: { meshes, firstOccluder: ray.intersectObject(object, true).find(hit => !hit.object.userData.formaSelectionOverlay)?.object.name,
      camera: camera.matrixWorld.toArray(), clippingEnabled: gl.localClippingEnabled, sceneObjects: scene.children.length } });
  });
  return null;
}
function Harness({ object }: { object: THREE.Group }) {
  const [mode, setMode] = useState<RenderMode>("solid"), [mounted, setMounted] = useState(true), [section, setSection] = useState(false);
  const selected = useWorkspace(state => state.selected), select = useWorkspace(state => state.setSelected);
  const light = useTheme() === "light";
  const plane = useMemo(() => ({ originMm: [0, 0, 20] as [number, number, number], normal: [0, 0, 1] as [number, number, number], deflectionMm: .05 }), []);
  return <div data-testid="appearance-harness" style={{ height: 760 }}>
    <div style={{ display: "flex", gap: 12, padding: 12 }}>
      <Select aria-label="Render mode" value={mode} onChange={event => setMode(event.target.value as RenderMode)}>
        {(["solid", "xray", "ghost", "transparent", "wireframe", "edges"] as const).map(value => <option key={value} value={value}>{t(value === "xray" ? "X-Ray" : value === "ghost" ? "Ghost" : value === "solid" ? "Solid" : value === "edges" ? "Solid + edges" : value === "wireframe" ? "Wireframe" : "Transparent")}</option>)}
      </Select>
      <button onClick={() => select("inner")} data-testid="select-inner">inner</button><button onClick={() => select("outer")} data-testid="select-outer">outer</button>
      <button onClick={() => setSection(value => !value)} data-testid="toggle-section">section</button>
      <button onClick={() => setMounted(value => !value)} data-testid="toggle-model">mount</button>
    </div>
    <Canvas camera={{ position: [85, 65, 85], fov: 40, near: .1, far: 1000 }} dpr={1}>
      <color attach="background" args={[light ? "#eef0f3" : "#1d2023"]} /><ambientLight intensity={1} /><directionalLight position={[50, 80, 50]} intensity={2} />
      <Probe object={object} />
      {mounted && <Model object={object} mode={mode} selected={selected} selectedFace={null} selectedEdge={null} selectionMode="body" revisionId="appearance" onPoint={() => {}} />}
      <SectionScene object={object} plane={plane} enabled={section} light={light} />
    </Canvas>
  </div>;
}
export async function mountRenderModeHarness(glb: string) {
  const object = await loadModel({ name: "nested.glb", kind: "model", size: 100, data: glb });
  useWorkspace.getState().setSelected(null);
  Object.assign(window, { __appearanceApplyTheme: applyTheme, __appearanceExport: async () => {
    const bytes = await (await exportMesh(object, "glb")).arrayBuffer();
    const view = new DataView(bytes), length = view.getUint32(12, true);
    return JSON.parse(new TextDecoder().decode(new Uint8Array(bytes, 20, length)));
  } });
  applyTheme("dark"); createRoot(document.getElementById("root")!).render(<Harness object={object} />);
}
