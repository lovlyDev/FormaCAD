import { describe, expect, it } from "vitest";
import * as THREE from "three";
import {
  cadEdges,
  edgeLengthMm,
  edgeRadiusMm,
  edgeOverlay,
} from "./edgeSelection";

describe("native CAD edge mapping", () => {
  it("reads exact BREP length independently of the display polyline", () => {
    const geometry = new THREE.BufferGeometry();
    geometry.userData.formaEdges = [
      {
        lengthMm: 10,
        radiusMm: null,
        semanticKey: "box-edge:x:ymin:zmin",
        points: [
          [0, 0, 0],
          [0.01, 0, 0],
        ],
      },
      {
        lengthMm: Math.PI * 5,
        radiusMm: 5,
        points: [
          [0, 0, 0],
          [0.005, 0, 0],
          [0.005, 0.005, 0],
        ],
      },
    ];
    const mesh = new THREE.Mesh(geometry);
    mesh.name = "housing";
    expect(cadEdges(mesh)).toHaveLength(2);
    expect(edgeLengthMm(mesh, 2)).toBe(Math.PI * 5);
    expect(edgeRadiusMm(mesh, 1)).toBeNull();
    expect(edgeRadiusMm(mesh, 2)).toBe(5);
    expect(cadEdges(mesh)?.[0].semanticKey).toBe("box-edge:x:ymin:zmin");
    const namedOverlay = edgeOverlay(mesh, cadEdges(mesh)![0], 1, true)!;
    expect(namedOverlay.userData.formaEdgeSemanticKey).toBe("box-edge:x:ymin:zmin");
    namedOverlay.geometry.dispose();
    (namedOverlay.material as THREE.Material).dispose();
    const overlay = edgeOverlay(mesh, cadEdges(mesh)![1], 2, true)!;
    expect(overlay.userData.formaEdgeOrdinal).toBe(2);
    expect(overlay.userData.formaBodyId).toBe("housing");
    expect(overlay.geometry.getAttribute("position").count).toBe(4);
    overlay.geometry.dispose();
    (overlay.material as THREE.Material).dispose();
    geometry.dispose();
  });

  it("rejects malformed metadata instead of selecting a wrong edge", () => {
    const geometry = new THREE.BufferGeometry();
    geometry.userData.formaEdges = [
      {
        lengthMm: NaN,
        points: [
          [0, 0, 0],
          [1, 0, 0],
        ],
      },
    ];
    const mesh = new THREE.Mesh(geometry);
    expect(cadEdges(mesh)).toBeNull();
    expect(edgeLengthMm(mesh, 1)).toBeNull();
    geometry.userData.formaEdges = [{ lengthMm: 10, semanticKey: "edge[3]", points: [[0, 0, 0], [1, 0, 0]] }];
    expect(cadEdges(mesh)).toBeNull();
    geometry.dispose();
  });
  it("keeps constructor circular references while a cylinder seam stays unreferenced", () => {
    const geometry=new THREE.BufferGeometry();
    const reference={schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"cylinder-edge:top",occurrencePath:["turned"]};
    geometry.userData.formaEdges=[{lengthMm:20*Math.PI,radiusMm:10,semanticKey:null,topologyRef:reference,points:[[10,0,5],[0,10,5]]},{lengthMm:5,radiusMm:null,topologyRef:null,points:[[10,0,0],[10,0,5]]}];
    const mesh=new THREE.Mesh(geometry);expect(cadEdges(mesh)?.[0].topologyRef).toEqual(reference);expect(cadEdges(mesh)?.[1].topologyRef).toBeUndefined();
    geometry.userData.formaEdges=[{lengthMm:20*Math.PI,topologyRef:{...reference,role:"cylinder-edge:seam"},points:[[10,0,5],[0,10,5]]}];expect(cadEdges(mesh)).toBeNull();geometry.dispose();
  });
});
