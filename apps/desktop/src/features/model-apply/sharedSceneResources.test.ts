import { expect, it, vi } from "vitest";
import * as THREE from "three";
import { disposeModel } from "../../lib/model";

it("releases shared geometry and original materials once per retired scene", () => {
  const geometry = new THREE.BoxGeometry(), material = new THREE.MeshStandardMaterial();
  const group = new THREE.Group();
  group.add(new THREE.Mesh(geometry, material), new THREE.Mesh(geometry, [material, material]));
  const disposeGeometry = vi.spyOn(geometry, "dispose"), disposeMaterial = vi.spyOn(material, "dispose");
  disposeModel(group);
  expect(disposeGeometry).toHaveBeenCalledTimes(1);
  expect(disposeMaterial).toHaveBeenCalledTimes(1);
});
