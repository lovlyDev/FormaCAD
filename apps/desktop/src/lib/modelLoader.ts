import * as THREE from "three";
import {STLLoader} from "three/examples/jsm/loaders/STLLoader.js";
import {OBJLoader} from "three/examples/jsm/loaders/OBJLoader.js";
import {GLTFLoader} from "three/examples/jsm/loaders/GLTFLoader.js";
import {ThreeMFLoader} from "three/examples/jsm/loaders/3MFLoader.js";
import {restoreCadTopology,hasCadTopology} from "./cadTopologyTransport";
import {native,readProjectFile} from "./api";
import {disposeModel} from "./model";
import {verifyFileSource} from "../features/project-sources/fileReadGuard";
import type {FolderSourceRef} from "../features/project-sources/contract";
import type {ProjectFile} from "../types";
export async function loadModel(
  file: ProjectFile,
  projectId?: string,
  source?: FolderSourceRef,
): Promise<THREE.Group> {
  file={...file};
  if(source){
    if(!native)throw new Error("PROJECT_FOLDER_UNSUPPORTED");
    if(!projectId)throw new Error("PROJECT_SNAPSHOT_CHANGED");
    file={...file,data:await readProjectFile(projectId,file.name,source)};
  }
  if (!source && !file.data && native && projectId)
    file = { ...file, data: await readProjectFile(projectId, file.name) };
  if (!file.data) throw new Error("Model content is missing.");
  const bytes = await (await fetch(file.data)).arrayBuffer();
  if(source&&bytes.byteLength!==file.size)throw new Error("PROJECT_SNAPSHOT_CHANGED");
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
      restoreCadTopology(o);
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
  group.userData.formaNativeCadPreview = hasCadTopology(group);
  if (!count) throw new Error("The file does not contain readable geometry.");
  const bounds = new THREE.Box3().setFromObject(group);
  if (
    ![...bounds.min.toArray(), ...bounds.max.toArray()].every(
      Number.isFinite,
    ) ||
    bounds.isEmpty()
  )
    throw new Error("Invalid model bounds.");
  // Native CAD previews keep CAD coordinates so viewport transforms and the
  // 3D cursor stay where the model is; other files are centred on the grid.
  if (!group.userData.formaNativeCadPreview) {
    const center = bounds.getCenter(new THREE.Vector3());
    group.position.set(-center.x, -bounds.min.y, -center.z);
  }
  group.updateMatrixWorld(true);
  if(source){try{await verifyFileSource(projectId!,file,source);}catch(cause){disposeModel(group);throw cause;}}
  return group;
}
