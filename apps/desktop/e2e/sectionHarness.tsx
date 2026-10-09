import { useEffect, useMemo } from "react";
import { createRoot } from "react-dom/client";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import * as THREE from "three";
import { loadModel } from "../src/lib/files";
import { applyTheme, useTheme } from "../src/lib/theme";
import { newProject, useWorkspace } from "../src/stores/workspace";
import { SectionControls } from "../src/features/viewer/sections/SectionControls";
import { SectionScene } from "../src/features/viewer/sections/SectionScene";
import { useSectionView } from "../src/features/viewer/sections/useSectionView";
import { getSectionPlane } from "../src/features/viewer/sections/sectionClipping";
import type { SectionReport } from "../src/features/viewer/sections/sectionApi";
import "../src/styles.css";

/** Browser-only probe: production sections and the ordinary GLB loader remain intact. */
function Inspect({ object }: { object: THREE.Group }) {
  const { gl, camera, scene } = useThree();
  const originals = useMemo(() => {
    const meshes = new Map<THREE.Mesh, THREE.Mesh["raycast"]>();
    const materials = new Map<THREE.Material, THREE.Plane[] | null>();
    object.traverse(node => {
      if (!(node instanceof THREE.Mesh)) return;
      meshes.set(node, node.raycast);
      for (const material of Array.isArray(node.material) ? node.material : [node.material]) materials.set(material, material.clippingPlanes);
    });
    return { meshes, materials };
  }, [object]);
  useEffect(() => {
    camera.lookAt(new THREE.Box3().setFromObject(object).getCenter(new THREE.Vector3()));
    camera.updateMatrixWorld();
  }, [camera, object]);
  useFrame(() => {
    object.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(object), center = box.getCenter(new THREE.Vector3()), size = box.getSize(new THREE.Vector3());
    const x = center.x + size.x * .3, z = center.z + size.z * .013;
    const lower = new THREE.Raycaster(new THREE.Vector3(x, box.min.y - 1, z), new THREE.Vector3(0, 1, 0), 0, 2);
    const upper = new THREE.Raycaster(new THREE.Vector3(x, box.max.y + 1, z), new THREE.Vector3(0, -1, 0), 0, 2);
    let positions: number[] = [], color: string | null = null;
    scene.traverse(node => {
      if (node instanceof THREE.LineSegments && node.renderOrder === 30) {
        positions = Array.from(node.geometry.getAttribute("position").array);
        color = (node.material as THREE.LineBasicMaterial).color.getHexString();
      }
    });
    Object.assign(window, { __sectionInspection: {
      clippingEnabled: gl.localClippingEnabled,
      allRaycastsRestored: [...originals.meshes].every(([mesh, original]) => mesh.raycast === original),
      allMaterialsRestored: [...originals.materials].every(([material, original]) => material.clippingPlanes === original),
      clipPlaneCount: [...originals.meshes.keys()].filter(mesh => getSectionPlane(mesh)).length,
      clippingNormal: [...originals.meshes.keys()].map(mesh => getSectionPlane(mesh)?.normal.toArray()).find(Boolean) ?? null,
      meshCount: originals.meshes.size,
      lowerHits: lower.intersectObject(object, true).length,
      upperHits: upper.intersectObject(object, true).length,
      positions, color, worldBounds: size.toArray(), camera: camera.matrixWorld.toArray(),
    } });
  });
  return null;
}

function Harness({ object }: { object: THREE.Group }) {
  const section = useSectionView(object, "forma.ui.e2e-sections", false);
  const light = useTheme() === "light";
  useEffect(() => {
    Object.assign(window, { __sectionState: { available: section.available, enabled: section.enabled, pending: section.pending, report: section.report, plane: section.plane } });
  }, [section]);
  return <div data-testid="section-harness" style={{ position: "relative", width: "100%", height: 760 }}>
    <button data-testid="open-section" onClick={() => section.setOpen(true)} style={{ position: "absolute", zIndex: 30, left: 14, top: 14 }}>Open section controls</button>
    <Canvas camera={{ position: [70, 55, 70], fov: 40, near: .1, far: 1000 }} dpr={1}>
      <color attach="background" args={[light ? "#eef0f3" : "#1d2023"]} />
      <ambientLight intensity={1} /><directionalLight position={[50, 80, 50]} intensity={2} />
      <primitive object={object} dispose={null} />
      <Inspect object={object} />
      <SectionScene object={object} plane={section.plane} enabled={section.enabled && section.valid} report={section.report} light={light} />
    </Canvas>
    <SectionControls section={section} />
  </div>;
}

export async function mountSectionHarness(fixture: { glb: string; document: { revisionId: string }; sourceSha256: string; sourceSize: number }) {
  const object = await loadModel({ name: "preview.glb", kind: "model", size: 100, data: fixture.glb });
  const project = newProject("Section QA", "blank", "mm", "codex");
  project.currentRevision = fixture.document.revisionId;
  project.revisions = [{ id: project.currentRevision, parent: null, createdAt: project.createdAt, prompt: "Unchanged baseline", source: "source.step", preview: "preview.glb", program: JSON.stringify(fixture.document), parameters: { kind: "blank", width: 1, depth: 1, height: 1, thickness: 1, holeDiameter: 0, holes: 0 } }];
  project.files = [{ name: "source.step", kind: "model", size: fixture.sourceSize, sha256: fixture.sourceSha256 }, { name: "preview.glb", kind: "model", size: 100, data: fixture.glb }];
  useWorkspace.getState().setProject(project);
  Object.assign(window, { __sectionBaseline: JSON.stringify(project), __sectionApplyTheme: applyTheme });
  applyTheme("dark");
  createRoot(document.getElementById("root")!).render(<Harness object={object} />);
}

export type SectionHarnessState = { available: boolean; enabled: boolean; pending: boolean; report?: SectionReport };
