import * as THREE from "three";
import { restoreExportAppearance } from "../../viewer/appearance/bodyAppearance";
import { stripViewerOverlays } from "../../viewer/stripViewerOverlays";
import { CAMERA_FOV, cameraPosition, frameQuaternion, type OrbitSpec } from "./orbitMath";

const BACKGROUND = { light: "#f5f7f9", dark: "#202326" } as const;

export interface OrbitStage {
  canvas: HTMLCanvasElement;
  /** Render frame `index` into `canvas`. */
  render: (index: number) => void;
  dispose: () => void;
}

/**
 * Off-screen turntable: a clone of the displayed model on its own renderer
 * with studio lights fixed to the camera, so the viewer and its camera stay
 * untouched while recording. Lights and colours follow the 3D view.
 */
export function createOrbitStage(source: THREE.Object3D, spec: OrbitSpec, theme: "light" | "dark"): OrbitStage {
  source.updateMatrixWorld(true);
  const model = source.clone(true);
  restoreExportAppearance(model);
  stripViewerOverlays(model);
  model.updateMatrixWorld(true);
  const bounds = new THREE.Box3().setFromObject(model);
  if (bounds.isEmpty() || ![...bounds.min.toArray(), ...bounds.max.toArray()].every(Number.isFinite))
    throw new Error("PLUGIN_CAPTURE_EMPTY");
  const sphere = bounds.getBoundingSphere(new THREE.Sphere());
  model.position.sub(sphere.center);
  const pivot = new THREE.Group();
  pivot.add(model);

  const scene = new THREE.Scene();
  const background = spec.background === "theme" ? theme : spec.background;
  scene.background = new THREE.Color(BACKGROUND[background]);
  scene.add(pivot);
  const camera = new THREE.PerspectiveCamera(CAMERA_FOV, spec.width / spec.height, sphere.radius / 100, sphere.radius * 400);
  camera.position.copy(cameraPosition(spec, sphere.radius));
  camera.lookAt(0, 0, 0);
  scene.add(camera);
  scene.add(new THREE.AmbientLight("#ffffff", 1.5));
  scene.add(new THREE.HemisphereLight("#dbe5ff", "#4d4234", 2));
  const key = new THREE.DirectionalLight("#ffffff", 3);
  key.position.set(0.5, 1, 0.6);
  const fill = new THREE.DirectionalLight("#c6d9ff", 2);
  fill.position.set(-0.9, 0.5, -0.4);
  camera.add(key, key.target, fill, fill.target);
  key.target.position.set(0, 0, -1);
  fill.target.position.set(0, 0, -1);

  const canvas = document.createElement("canvas");
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
  renderer.setPixelRatio(1);
  renderer.setSize(spec.width, spec.height, false);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  return {
    canvas,
    render: (index) => {
      pivot.quaternion.copy(frameQuaternion(spec, index));
      renderer.render(scene, camera);
    },
    dispose: () => {
      model.removeFromParent();
      renderer.dispose();
      renderer.forceContextLoss();
    },
  };
}
