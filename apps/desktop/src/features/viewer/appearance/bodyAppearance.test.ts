import { describe, expect, it, vi } from "vitest";
import * as THREE from "three";
import { applyBodyAppearance, restoreExportAppearance } from "./bodyAppearance";
import { addBodyOutlines } from "./bodyOutlines";
import { setSectionPlane } from "../sections/sectionClipping";
function bodies() {
  const group = new THREE.Group();
  for (const name of ["outer", "inner"]) {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 10), new THREE.MeshStandardMaterial({ color: "#89abcd" }));
    mesh.name = name; group.add(mesh);
  }
  return group;
}
describe("owned viewer appearances", () => {
  it("X-Ray reveals hidden edges without writing surface depth or changing source materials", () => {
    const group = bodies(), mesh = group.children[0] as THREE.Mesh;
    const original = mesh.material as THREE.MeshStandardMaterial;
    const geometry = mesh.geometry;
    const cleanup = applyBodyAppearance(group, "xray", null, false);
    const outlines = addBodyOutlines(group, "xray", null, false);
    const material = mesh.material as THREE.MeshStandardMaterial;
    expect(material).not.toBe(original);
    expect(material.opacity).toBe(.12); expect(material.depthWrite).toBe(false);
    expect(material.side).toBe(THREE.DoubleSide);
    const line = mesh.children[0] as THREE.LineSegments;
    expect((line.material as THREE.Material).depthTest).toBe(false);
    expect(line.userData.formaSelectionOverlay).toBe(true);
    expect(line.raycast(new THREE.Raycaster(), [])).toBeUndefined();
    expect(original.opacity).toBe(1); expect(original.transparent).toBe(false);
    outlines(); cleanup();
    expect(mesh.material).toBe(original); expect(mesh.geometry).toBe(geometry); expect(mesh.children).toHaveLength(0);
  });
  it.each([false, true])("Ghost tracks the selected body and theme=%s without dimming it", light => {
    const group = bodies();
    for (const selected of ["outer", "inner", null]) {
      const cleanup = applyBodyAppearance(group, "ghost", selected, light);
      for (const mesh of group.children as THREE.Mesh[]) {
        const material = mesh.material as THREE.MeshStandardMaterial;
        expect(material.opacity).toBe(mesh.name === selected ? 1 : .09);
        expect(material.depthWrite).toBe(mesh.name === selected);
        if (mesh.name === selected) expect(material.color.getHexString()).toBe(light ? "7c3aed" : "a78bfa");
      }
      cleanup();
    }
  });
  it("cleans repeated mode changes once, including material arrays and overlays", () => {
    const group = bodies(), mesh = group.children[0] as THREE.Mesh;
    const first = mesh.material as THREE.Material, second = first.clone();
    mesh.material = [first, second];
    const overlay = new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshBasicMaterial());
    overlay.userData.formaSelectionOverlay = true; mesh.add(overlay);
    const sourceDispose = vi.spyOn(first, "dispose"), overlayMaterial = overlay.material;
    for (let pass = 0; pass < 10; pass++) {
      const cleanup = applyBodyAppearance(group, pass % 2 ? "ghost" : "xray", "inner", false);
      const owned = (mesh.material as THREE.Material[]).map(material => vi.spyOn(material, "dispose"));
      const outlines = addBodyOutlines(group, "xray", null, false);
      const line = mesh.children.find(node => node.userData.formaAppearanceOutline) as THREE.LineSegments;
      const disposeGeometry = vi.spyOn(line.geometry, "dispose"), disposeLine = vi.spyOn(line.material as THREE.Material, "dispose");
      outlines(); outlines(); cleanup(); cleanup();
      owned.forEach(dispose => expect(dispose).toHaveBeenCalledTimes(1));
      expect(disposeGeometry).toHaveBeenCalledTimes(1); expect(disposeLine).toHaveBeenCalledTimes(1);
      expect(mesh.material).toEqual([first, second]); expect(mesh.children).toEqual([overlay]);
      expect(overlay.material).toBe(overlayMaterial);
    }
    expect(sourceDispose).not.toHaveBeenCalled();
  });
  it("retains a pre-existing clipping plane and restores its material reference", () => {
    const group = bodies(), mesh = group.children[0] as THREE.Mesh;
    const original = mesh.material as THREE.Material;
    original.clippingPlanes = [new THREE.Plane(new THREE.Vector3(0, 1, 0), -2)];
    const cleanup = applyBodyAppearance(group, "ghost", "inner", false);
    expect((mesh.material as THREE.Material).clippingPlanes).toEqual(original.clippingPlanes);
    cleanup(); expect(mesh.material).toBe(original);
  });
  it("restores export clone materials while the live Ghost scene remains dimmed", () => {
    const group = bodies(), mesh = group.children[0] as THREE.Mesh;
    const source = mesh.material;
    const cleanup = applyBodyAppearance(group, "ghost", "inner", true);
    const display = mesh.material;
    const exported = group.clone(true);
    restoreExportAppearance(exported);
    expect((exported.children[0] as THREE.Mesh).material).toBe(source);
    expect(mesh.material).toBe(display);
    expect((mesh.material as THREE.Material).opacity).toBe(.09);
    expect((source as THREE.Material).opacity).toBe(1);
    expect(exported.userData).toEqual({});
    cleanup();
  });
  it("does not inherit a temporary section plane, preserving independent clipping", () => {
    const group = bodies(), mesh = group.children[0] as THREE.Mesh;
    const source = mesh.material as THREE.Material;
    const temporary = new THREE.Plane(new THREE.Vector3(0, 1, 0), -2);
    const persistent = new THREE.Plane(new THREE.Vector3(1, 0, 0), -3);
    setSectionPlane(mesh, temporary); source.clippingPlanes = [temporary, persistent];
    const cleanup = applyBodyAppearance(group, "xray", null, false);
    expect((mesh.material as THREE.Material).clippingPlanes).toEqual([persistent]);
    cleanup(); expect(source.clippingPlanes).toEqual([temporary, persistent]);
    setSectionPlane(mesh, null);
  });
});
