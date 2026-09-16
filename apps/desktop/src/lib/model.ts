import * as THREE from "three";
import { z } from "zod";
import type { Parameters } from "../types";
export function holeLimit(p: Parameters) {
  if (p.kind !== "box") return Math.min(p.width, p.depth) / 4;
  if (p.holes <= 1) return Math.min(p.width, p.depth) * 0.9;
  const columns = Math.ceil(p.holes / 2);
  return Math.min(
    p.width * 0.36,
    p.depth * 0.44,
    columns > 1 ? ((p.width * 0.64) / (columns - 1)) * 0.9 : p.depth * 0.5,
  );
}
export function geometrySignature(p: Parameters) {
  const v = { ...p };
  if (v.kind === "box" || v.kind === "cylinder") v.thickness = 0;
  if (v.kind === "plate") v.height = 0;
  if (v.kind === "cylinder") {
    v.depth = 0;
    v.holes = 0;
  }
  if (v.kind === "enclosure") {
    v.holes = 0;
    v.holeDiameter = 0;
  }
  if (v.kind !== "cylinder" && (!v.holes || !v.holeDiameter)) {
    v.holes = 0;
    v.holeDiameter = 0;
  }
  return JSON.stringify(v);
}
export const parameterSchema = z
  .object({
    kind: z.enum(["box", "blank", "bracket", "enclosure", "plate", "cylinder"]),
    width: z.number().min(1).max(2000),
    depth: z.number().min(1).max(2000),
    height: z.number().min(1).max(2000),
    thickness: z.number().min(0.2).max(100),
    holeDiameter: z.number().min(0).max(200),
    holes: z.number().int().min(0).max(16),
  })
  .superRefine((p, ctx) => {
    if (p.thickness * 2 >= Math.min(p.width, p.depth, p.height))
      ctx.addIssue({
        code: "custom",
        message:
          "Wall thickness must be less than half the smallest dimension.",
      });
    if (p.holeDiameter > holeLimit(p))
      ctx.addIssue({
        code: "custom",
        message: "Holes are too large for this part.",
      });
  });
export function buildModel(p: Parameters): THREE.Group {
  parameterSchema.parse(p);
  const group = new THREE.Group();
  group.name = "Assembly";
  const material = new THREE.MeshStandardMaterial({
    color: "#bec8d1",
    metalness: 0.58,
    roughness: 0.34,
  });
  const add = (
    geometry: THREE.BufferGeometry,
    name: string,
    pos: [number, number, number] = [0, 0, 0],
  ) => {
    const m = new THREE.Mesh(geometry, material);
    m.name = name;
    m.userData.formaLabel = `body.${name}`;
    m.position.set(...pos);
    m.castShadow = true;
    m.receiveShadow = true;
    group.add(m);
    return m;
  };
  const plate = (w: number, d: number, thick: number, holes: boolean) => {
    const s = new THREE.Shape();
    s.moveTo(-w / 2, -d / 2);
    s.lineTo(w / 2, -d / 2);
    s.lineTo(w / 2, d / 2);
    s.lineTo(-w / 2, d / 2);
    s.closePath();
    if (holes && p.holeDiameter > 0)
      for (let i = 0; i < p.holes; i++) {
        const columns = Math.ceil(p.holes / 2);
        const x =
          columns === 1
            ? 0
            : -w * 0.32 + ((i % columns) * w * 0.64) / (columns - 1);
        const y =
          p.kind === "box" && p.holes === 1
            ? 0
            : i < columns
              ? -d * 0.28
              : d * 0.28;
        const h = new THREE.Path();
        h.absarc(x, y, p.holeDiameter / 2, 0, Math.PI * 2, true);
        s.holes.push(h);
      }
    return new THREE.ExtrudeGeometry(s, {
      depth: thick,
      bevelEnabled: false,
      curveSegments: 48,
    });
  };
  if (p.kind === "blank") return group;
  if (p.kind === "box") {
    const block = add(plate(p.width, p.depth, p.height, true), "Block");
    block.rotation.x = -Math.PI / 2;
  } else if (p.kind === "cylinder") {
    const outer = new THREE.Shape();
    outer.absarc(0, 0, p.width / 2, 0, Math.PI * 2, false);
    if (p.holeDiameter > 0) {
      const h = new THREE.Path();
      h.absarc(0, 0, p.holeDiameter / 2, 0, Math.PI * 2, true);
      outer.holes.push(h);
    }
    const mesh = add(
      new THREE.ExtrudeGeometry(outer, {
        depth: p.height,
        bevelEnabled: false,
        curveSegments: 64,
      }),
      "Cylinder",
    );
    mesh.rotation.x = -Math.PI / 2;
  } else {
    const base = add(
      plate(p.width, p.depth, p.thickness, p.kind !== "enclosure"),
      "Base",
    );
    base.rotation.x = -Math.PI / 2;
    if (p.kind === "bracket")
      add(
        new THREE.BoxGeometry(p.width, p.height - p.thickness, p.thickness),
        "Upright",
        [0, (p.height + p.thickness) / 2, -p.depth / 2 + p.thickness / 2],
      );
    if (p.kind === "enclosure") {
      const y = (p.height + p.thickness) / 2;
      add(
        new THREE.BoxGeometry(p.width, p.height - p.thickness, p.thickness),
        "Back",
        [0, y, -p.depth / 2 + p.thickness / 2],
      );
      add(
        new THREE.BoxGeometry(p.width, p.height - p.thickness, p.thickness),
        "Front",
        [0, y, p.depth / 2 - p.thickness / 2],
      );
      add(
        new THREE.BoxGeometry(
          p.thickness,
          p.height - p.thickness,
          p.depth - 2 * p.thickness,
        ),
        "Left",
        [-p.width / 2 + p.thickness / 2, y, 0],
      );
      add(
        new THREE.BoxGeometry(
          p.thickness,
          p.height - p.thickness,
          p.depth - 2 * p.thickness,
        ),
        "Right",
        [p.width / 2 - p.thickness / 2, y, 0],
      );
    }
  }
  group.updateMatrixWorld(true);
  return group;
}
export function inspectModel(object: THREE.Object3D) {
  object.updateMatrixWorld(true);
  const bounds = new THREE.Box3().setFromObject(object);
  const size = bounds.isEmpty()
    ? new THREE.Vector3()
    : bounds.getSize(new THREE.Vector3());
  let triangles = 0,
    area = 0,
    volume = 0,
    bodies = 0;
  const a = new THREE.Vector3(),
    b = new THREE.Vector3(),
    c = new THREE.Vector3(),
    ab = new THREE.Vector3(),
    ac = new THREE.Vector3();
  object.traverse((o) => {
    if (!(o instanceof THREE.Mesh)) return;
    bodies++;
    const g = o.geometry as THREE.BufferGeometry;
    const pos = g.getAttribute("position");
    if (!pos) return;
    const count = g.index?.count ?? pos.count;
    triangles += Math.floor(count / 3);
    for (let i = 0; i + 2 < count; i += 3) {
      const idx = (n: number) => g.index?.getX(n) ?? n;
      a.fromBufferAttribute(pos, idx(i)).applyMatrix4(o.matrixWorld);
      b.fromBufferAttribute(pos, idx(i + 1)).applyMatrix4(o.matrixWorld);
      c.fromBufferAttribute(pos, idx(i + 2)).applyMatrix4(o.matrixWorld);
      area += ab.subVectors(b, a).cross(ac.subVectors(c, a)).length() / 2;
      volume += a.dot(ab.copy(b).cross(c)) / 6;
    }
  });
  return {
    size: size.toArray(),
    triangles,
    bodies,
    area,
    volume: Math.abs(volume),
  };
}
export function disposeModel(object: THREE.Object3D) {
  object.traverse((o) => {
    if (o instanceof THREE.Mesh) {
      o.geometry.dispose();
      const mats = Array.isArray(o.material) ? o.material : [o.material];
      mats.forEach((m) => m.dispose());
    }
  });
}
