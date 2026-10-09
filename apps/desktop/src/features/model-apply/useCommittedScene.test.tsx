import { useEffect } from "react";
import { render, waitFor, act } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import * as THREE from "three";
import { newProject } from "../../stores/workspace";
import { defaults } from "../../types";
import { useCommittedScene } from "./useCommittedScene";
import { useBodyAppearance } from "../viewer/appearance/useBodyAppearance";
import { setSectionPlane } from "../viewer/sections/sectionClipping";
const mock = vi.hoisted(() => ({ load: vi.fn() }));
vi.mock("../../lib/files", () => ({ loadModel: mock.load }));
it.each(["xray", "ghost"] as const)("retires %s scene after borrowed appearance and clipping cleanups, exactly once", async mode => {
  const group = new THREE.Group(), original = new THREE.MeshStandardMaterial(), geometry = new THREE.BoxGeometry();
  const mesh = new THREE.Mesh(geometry, original); group.add(mesh);
  const originalDispose = vi.spyOn(original, "dispose"), geometryDispose = vi.spyOn(geometry, "dispose");
  const next = new THREE.Group(); next.add(new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshStandardMaterial()));
  mock.load.mockReset().mockResolvedValueOnce(group).mockResolvedValueOnce(next);
  const project = newProject("Scene", "blank", "mm", "codex");
  const revision = { id: "base", parent: null, createdAt: project.createdAt, prompt: "baseline", parameters: defaults, program: "{}", source: "source.step", preview: "base.glb" };
  project.currentRevision = revision.id; project.revisions = [revision]; project.files = [{ name: "base.glb", kind: "model", size: 100, data: "base" }];
  const empty = new THREE.Group();
  function Host({ value = project }) {
    const r = value.revisions.find(item => item.id === value.currentRevision);
    const scene = useCommittedScene(value, r), object = scene.object ?? empty;
    useBodyAppearance(object, mode, null, false);
    useEffect(() => {
      object.traverse(node => { if (node instanceof THREE.Mesh) { const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0)); setSectionPlane(node, plane); (node.material as THREE.Material).clippingPlanes = [plane]; } });
      return () => object.traverse(node => { if (node instanceof THREE.Mesh) { setSectionPlane(node, null); (node.material as THREE.Material).clippingPlanes = null; } });
    }, [object]);
    return <div data-testid="scene">{scene.displayed?.revisionId}</div>;
  }
  const view = render(<Host />); await waitFor(() => expect(view.getByTestId("scene")).toHaveTextContent("base"));
  const cloneDispose = vi.spyOn(mesh.material as THREE.Material, "dispose");
  const updated = { ...project, currentRevision: "next", revisions: [...project.revisions, { ...revision, id: "next", preview: "next.glb" }], files: [...project.files, { name: "next.glb", kind: "model" as const, size: 100, data: "next" }] };
  view.rerender(<Host value={updated} />);
  await waitFor(() => expect(view.getByTestId("scene")).toHaveTextContent("next"));
  await act(async () => { await Promise.resolve(); });
  expect(cloneDispose).toHaveBeenCalledTimes(1); expect(originalDispose).toHaveBeenCalledTimes(1); expect(geometryDispose).toHaveBeenCalledTimes(1);
  expect(mesh.material).toBe(original); expect(original.clippingPlanes).toBeNull();
  view.unmount();
});
