import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../lib/api";
import { currentParameters, newProject, useWorkspace } from "./workspace";
import { defaults } from "../types";
import { listProjects } from "../lib/api";
describe("project history", () => {
  beforeEach(() => {
    localStorage.clear();
    useWorkspace.getState().setProject(null);
  });
  it("restores by appending without mutating older revisions", async () => {
    const p = newProject("Bracket", "bracket", "mm", "codex");
    useWorkspace.getState().setProject(p);
    await useWorkspace.getState().revise(defaults, "Initial legacy model");
    await useWorkspace.getState().revise({ ...defaults, width: 140 }, "Wider");
    await useWorkspace.getState().revise(defaults, "Restored revision 1");
    const result = useWorkspace.getState().project!;
    expect(result.revisions).toHaveLength(3);
    expect(result.revisions[1].parameters.width).toBe(140);
    expect(currentParameters(result).width).toBe(120);
    expect(result.revisions[2].parent).toBe(result.revisions[1].id);
    expect((await listProjects())[0].revisions).toHaveLength(3);
  });
  it("keeps an empty project empty", () => {
    const p = newProject("Empty", "bracket", "inch", "claude");
    expect(p.currentRevision).toBeNull();
    expect(p.revisions).toHaveLength(0);
  });
  it("preserves CAD selection for a no-op but clears it for another project with the same revision", () => {
    const a = { ...newProject("A", "blank", "mm", "codex"), currentRevision: "shared" };
    useWorkspace.getState().setProject(a);
    useWorkspace.getState().setSelectedEdge({ bodyId: "body", edgeOrdinal: 1, revisionId: "shared" });
    const selected = useWorkspace.getState().selectedEdge;
    useWorkspace.getState().setProject({ ...a, name: "A renamed" });
    expect(useWorkspace.getState().selectedEdge).toBe(selected);
    useWorkspace.getState().setProject({ ...a, id: "different" });
    expect(useWorkspace.getState().selectedEdge).toBeNull();
  });
  it("a delayed metadata save cannot switch the active project", async () => {
    const a = newProject("A", "blank", "mm", "codex"), b = newProject("B", "blank", "mm", "codex");
    useWorkspace.getState().setProject(a);
    let finish!: (value: typeof a) => void;
    const save = vi.spyOn(api, "saveProject").mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    const pending = useWorkspace.getState().update(a);
    useWorkspace.getState().setProject(b); finish(a); await pending;
    expect(useWorkspace.getState().project?.id).toBe(b.id); save.mockRestore();
  });
  it("clears a face selection when the revision changes", async () => {
    const project = newProject("Housing", "box", "mm", "codex");
    useWorkspace.getState().setProject(project);
    await useWorkspace.getState().revise(defaults, "Initial model");
    const first = useWorkspace.getState().project!;
    useWorkspace.getState().setSelected("housing");
    useWorkspace.getState().setSelectedFace({
      bodyId: "housing",
      faceOrdinal: 2,
      revisionId: first.currentRevision!,
    });
    await useWorkspace
      .getState()
      .revise({ ...defaults, width: 140 }, "Changed width");
    expect(useWorkspace.getState().selectedFace).toBeNull();
  });
  it("clears an edge selection when the revision changes", async () => {
    const project = newProject("Housing", "box", "mm", "codex");
    useWorkspace.getState().setProject(project);
    await useWorkspace.getState().revise(defaults, "Initial model");
    const first = useWorkspace.getState().project!;
    useWorkspace.getState().setSelected("housing");
    useWorkspace.getState().setSelectedEdge({
      bodyId: "housing",
      edgeOrdinal: 3,
      revisionId: first.currentRevision!,
    });
    await useWorkspace
      .getState()
      .revise({ ...defaults, width: 140 }, "Changed width");
    expect(useWorkspace.getState().selectedEdge).toBeNull();
  });
  it("does not silently reset corrupted browser storage", async () => {
    localStorage.setItem("forma.projects.v1", "not-json");
    await expect(listProjects()).rejects.toThrow("cannot be read");
    expect(localStorage.getItem("forma.projects.v1")).toBe("not-json");
  });
});
