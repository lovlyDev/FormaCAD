import * as THREE from "three";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { ThreeMFLoader } from "three/examples/jsm/loaders/3MFLoader.js";
import type { ProjectFile } from "../types";
import { native, readProjectFile } from "./api";
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
export async function loadModel(
  file: ProjectFile,
  projectId?: string,
): Promise<THREE.Group> {
  if (!file.data && native && projectId)
    file = { ...file, data: await readProjectFile(projectId, file.name) };
  if (!file.data) throw new Error("Model content is missing.");
  const bytes = await (await fetch(file.data)).arrayBuffer();
  const ext = file.name.split(".").pop()?.toLowerCase();
  let group: THREE.Group;
  if (ext === "stl") {
    group = new THREE.Group();
    const geometry = new STLLoader().parse(bytes);
    geometry.computeVertexNormals();
    const mesh = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial());
    mesh.name = file.name;
    group.add(mesh);
  } else if (ext === "obj") {
    const text = new TextDecoder().decode(bytes);
    if (/^mtllib\s/m.test(text))
      throw new Error(
        "Import a self-contained OBJ without external material libraries.",
      );
    group = new OBJLoader().parse(text);
  } else if (ext === "glb") {
    const view = new DataView(bytes);
    if (bytes.byteLength < 20) throw new Error("Invalid GLB file.");
    const len = view.getUint32(12, true);
    const json = JSON.parse(
      new TextDecoder().decode(new Uint8Array(bytes, 20, len)),
    ) as { buffers?: { uri?: string }[]; images?: { uri?: string }[] };
    if (
      [...(json.buffers ?? []), ...(json.images ?? [])].some(
        (x) => x.uri && !x.uri.startsWith("data:"),
      )
    )
      throw new Error("External GLB resources are not allowed.");
    const gltf = await new GLTFLoader().parseAsync(bytes, "");
    group = new THREE.Group();
    group.add(gltf.scene);
    // glTF uses meters. The CAD workspace and all internal dimensions use millimeters.
    group.scale.setScalar(1000);
  } else if (ext === "3mf") {
    group = new ThreeMFLoader().parse(bytes);
    group.rotateX(-Math.PI / 2);
  } else
    throw new Error(
      "STEP needs the native CAD converter. DXF is stored as a drawing attachment.",
    );
  let count = 0;
  group.traverse((o) => {
    if (o instanceof THREE.Mesh) {
      count += o.geometry.getAttribute("position")?.count ?? 0;
      if (!o.geometry.getAttribute("normal")) o.geometry.computeVertexNormals();
      o.material = new THREE.MeshStandardMaterial({
        color: "#b4bfc8",
        metalness: 0.5,
        roughness: 0.35,
      });
      if (!o.name) o.name = `Body ${++count}`;
    }
  });
  if (!count) throw new Error("The file does not contain readable geometry.");
  const bounds = new THREE.Box3().setFromObject(group);
  if (
    ![...bounds.min.toArray(), ...bounds.max.toArray()].every(
      Number.isFinite,
    ) ||
    bounds.isEmpty()
  )
    throw new Error("Invalid model bounds.");
  const center = bounds.getCenter(new THREE.Vector3());
  group.position.set(-center.x, -bounds.min.y, -center.z);
  group.updateMatrixWorld(true);
  return group;
}
export async function exportMesh(
  object: THREE.Group,
  format: string,
): Promise<Blob> {
  object.updateMatrixWorld(true);
  if (format === "3mf") {
    const { encodeThreeMf } = await import("./threeMf");
    return new Blob([new Uint8Array(encodeThreeMf(object))], { type: "model/3mf" });
  }
  if (format === "stl") {
    const { STLExporter } =
      await import("three/examples/jsm/exporters/STLExporter.js");
    return new Blob([new STLExporter().parse(object)], { type: "model/stl" });
  }
  if (format === "obj") {
    const { OBJExporter } =
      await import("three/examples/jsm/exporters/OBJExporter.js");
    return new Blob([new OBJExporter().parse(object)], { type: "text/plain" });
  }
  if (format === "glb") {
    const { GLTFExporter } =
      await import("three/examples/jsm/exporters/GLTFExporter.js");
    const exported = object.clone(true);
    exported.scale.multiplyScalar(0.001);
    exported.position.multiplyScalar(0.001);
    exported.updateMatrixWorld(true);
    const lines: THREE.Object3D[] = [];
    exported.traverse((o) => {
      if (o instanceof THREE.LineSegments) lines.push(o);
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
