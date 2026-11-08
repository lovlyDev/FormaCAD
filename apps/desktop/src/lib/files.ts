import { preserveCadTopology } from "./cadTopologyTransport";
import * as THREE from "three";
import type { ProjectFile } from "../types";
import { isolateExportBody } from "./isolateExportBody";
import { stripViewerOverlays } from "../features/viewer/stripViewerOverlays";
import { restoreExportAppearance } from "../features/viewer/appearance/bodyAppearance";
export async function readFile(file: File): Promise<ProjectFile> {
  if (file.size > 40 * 1024 * 1024)
    throw new Error("This import is limited to 40 MB. Use a smaller mesh.");
  const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
  if (
    ![
      "stl",
      "obj",
      "glb",
      "3mf",
      "png",
      "jpg",
      "jpeg",
      "webp",
      "pdf",
      "step",
      "stp",
      "dxf",
    ].includes(ext)
  )
    throw new Error("Unsupported file format.");
  const data = await new Promise<string>((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(String(r.result));
    r.onerror = () => reject(new Error("Could not read the selected file."));
    r.readAsDataURL(file);
  });
  return {
    name: file.name,
    size: file.size,
    kind: ["png", "jpg", "jpeg", "webp"].includes(ext)
      ? "image"
      : ["pdf", "dxf"].includes(ext)
        ? "drawing"
        : "model",
    data,
  };
}
export {loadModel} from "./modelLoader";
export async function exportMesh(
  object: THREE.Group,
  format: string,
  bodyId: string | null = null,
): Promise<Blob> {
  object.updateMatrixWorld(true);
  const exportObject = bodyId ? isolateExportBody(object, bodyId) : object.clone(true);
  restoreExportAppearance(exportObject);
  stripViewerOverlays(exportObject);
  exportObject.updateMatrixWorld(true);
  if (format === "3mf") {
    const { encodeThreeMf } = await import("./threeMf");
    return new Blob([new Uint8Array(encodeThreeMf(exportObject))], { type: "model/3mf" });
  }
  if (format === "stl") {
    const { STLExporter } =
      await import("three/examples/jsm/exporters/STLExporter.js");
    return new Blob([new STLExporter().parse(exportObject)], { type: "model/stl" });
  }
  if (format === "obj") {
    const { OBJExporter } =
      await import("three/examples/jsm/exporters/OBJExporter.js");
    return new Blob([new OBJExporter().parse(exportObject)], { type: "text/plain" });
  }
  if (format === "glb") {
    const { GLTFExporter } =
      await import("three/examples/jsm/exporters/GLTFExporter.js");
    const exported = exportObject.clone(true);
    exported.scale.multiplyScalar(0.001);
    exported.position.multiplyScalar(0.001);
    exported.updateMatrixWorld(true);
    const lines: THREE.Object3D[] = [];
    exported.traverse((o) => {
      if (o instanceof THREE.LineSegments) lines.push(o);
      if (o instanceof THREE.Mesh) preserveCadTopology(o);
    });
    lines.forEach((o) => o.removeFromParent());
    const result = await new GLTFExporter().parseAsync(exported, {
      binary: true,
      onlyVisible: true,
    });
    if (!(result instanceof ArrayBuffer)) throw new Error("GLB export failed.");
    return new Blob([result], { type: "model/gltf-binary" });
  }
  throw new Error("This export format needs the native CAD kernel.");
}

export function download(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}
