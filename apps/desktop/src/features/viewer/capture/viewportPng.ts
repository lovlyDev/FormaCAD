import { Vector2, Vector4, type Camera, type Scene, type WebGLRenderer } from "three";

export type ViewportPngOptions = {
  scale: 1 | 2 | 3;
  transparent: boolean;
};

export function viewportPng(
  gl: WebGLRenderer,
  scene: Scene,
  camera: Camera,
  { scale, transparent }: ViewportPngOptions,
): string {
  const size = gl.getSize(new Vector2());
  const buffer = gl.getDrawingBufferSize(new Vector2());
  const width = buffer.x * scale;
  const height = buffer.y * scale;
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) ||
      width < 1 || height < 1 || width > 4096 || height > 4096) {
    throw new Error("The snapshot exceeds 4096 pixels on one side. Choose a smaller scale.");
  }
  const pixelRatio = gl.getPixelRatio();
  const background = scene.background;
  const clearAlpha = gl.getClearAlpha();
  const viewport = gl.getViewport(new Vector4());
  const scissor = gl.getScissor(new Vector4());
  const scissorTest = gl.getScissorTest();
  try {
    if (transparent) {
      scene.background = null;
      gl.setClearAlpha(0);
    }
    if (scale !== 1) {
      gl.setPixelRatio(1);
      gl.setSize(width, height, false);
    }
    gl.setScissorTest(false);
    gl.render(scene, camera);
    const data = gl.domElement.toDataURL("image/png");
    if (!data.startsWith("data:image/png;base64,")) throw new Error("Could not capture the 3D view.");
    return data;
  } finally {
    scene.background = background;
    gl.setClearAlpha(clearAlpha);
    if (scale !== 1) {
      gl.setSize(size.x, size.y, false);
      gl.setPixelRatio(pixelRatio);
    }
    gl.setViewport(viewport);
    gl.setScissor(scissor);
    gl.setScissorTest(scissorTest);
    gl.render(scene, camera);
  }
}
