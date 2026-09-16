import * as THREE from "three";
import { zipSync, strToU8 } from "three/examples/jsm/libs/fflate.module.js";

const encode = (text: string) => new Uint8Array(strToU8(text));

// 3MF Core: OPC package, millimeter coordinates, Z-up build space.
export function encodeThreeMf(object: THREE.Group): Uint8Array {
  object.updateMatrixWorld(true);
  const resources: string[] = [],
    build: string[] = [];
  object.traverseVisible((node) => {
    if (!(node instanceof THREE.Mesh)) return;
    const position = node.geometry.getAttribute("position"),
      index = node.geometry.index;
    const vertices: string[] = [],
      triangles: string[] = [];
    const unique = new Map<string, number>(),
      mapped: number[] = [];
    const point = new THREE.Vector3();
    for (let i = 0; i < position.count; i++) {
      point.fromBufferAttribute(position, i).applyMatrix4(node.matrixWorld);
      const xyz = [point.x, -point.z, point.y];
      if (!xyz.every(Number.isFinite))
        throw new Error("Invalid vertex in model.");
      const key = xyz.join(",");
      let id = unique.get(key);
      if (id === undefined) {
        id = vertices.length;
        unique.set(key, id);
        vertices.push(`<vertex x="${xyz[0]}" y="${xyz[1]}" z="${xyz[2]}"/>`);
      }
      mapped.push(id);
    }
    const count = index?.count ?? position.count;
    const mirrored = node.matrixWorld.determinant() < 0;
    for (let i = 0; i + 2 < count; i += 3) {
      const v = [0, 1, 2].map(
        (offset) => mapped[index ? index.getX(i + offset) : i + offset],
      );
      if (new Set(v).size < 3) continue;
      if (mirrored) [v[1], v[2]] = [v[2], v[1]];
      triangles.push(`<triangle v1="${v[0]}" v2="${v[1]}" v3="${v[2]}"/>`);
    }
    if (!triangles.length) return;
    const id = resources.length + 1;
    resources.push(
      `<object id="${id}" type="model"><mesh><vertices>${vertices.join("")}</vertices><triangles>${triangles.join("")}</triangles></mesh></object>`,
    );
    build.push(`<item objectid="${id}"/>`);
  });
  if (!build.length) throw new Error("No visible geometry to export.");
  return zipSync({
    "[Content_Types].xml": encode(
      '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>',
    ),
    "_rels/.rels": encode(
      '<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>',
    ),
    "3D/3dmodel.model": encode(
      `<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><resources>${resources.join("")}</resources><build>${build.join("")}</build></model>`,
    ),
  });
}
