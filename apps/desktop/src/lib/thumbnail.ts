import * as THREE from "three";

/** Render one consistent isometric preview, independent of the user's camera and theme. */
export function renderThumbnail(model: THREE.Group): string | null {
  const bounds = new THREE.Box3().setFromObject(model);
  if (bounds.isEmpty() || ![...bounds.min.toArray(), ...bounds.max.toArray()].every(Number.isFinite)) return null;

  const canvas = document.createElement("canvas");
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
  try {
    renderer.setSize(480, 270, false);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#202326");
    scene.add(model);
    const center = bounds.getCenter(new THREE.Vector3());
    const size = bounds.getSize(new THREE.Vector3());
    const distance = Math.max(size.length() * 1.35, 10);
    const camera = new THREE.PerspectiveCamera(36, 16 / 9, 0.01, distance * 10);
    camera.position.copy(center).addScaledVector(new THREE.Vector3(1.3, 0.95, 1.35).normalize(), distance);
    camera.lookAt(center);
    scene.add(new THREE.HemisphereLight("#f6f8ff", "#495363", 2.4));
    const light = new THREE.DirectionalLight("#ffffff", 3);
    light.position
      .copy(center)
      .add(new THREE.Vector3(size.length(), size.length() * 2, size.length()));
    scene.add(light);
    const grid = new THREE.GridHelper(
      Math.max(size.x, size.z, 40) * 2,
      12,
      new THREE.Color("#45505c"),
      new THREE.Color("#303840"),
    );
    grid.position.y = bounds.min.y - Math.max(size.y * 0.03, 0.2);
    scene.add(grid);
    renderer.render(scene, camera);
    return canvas.toDataURL("image/jpeg", 0.78);
  } finally {
    model.removeFromParent();
    renderer.dispose();
    renderer.forceContextLoss();
  }
}
