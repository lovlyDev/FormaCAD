import { beforeEach, describe, expect, it } from "vitest";
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
  it("does not silently reset corrupted browser storage", async () => {
    localStorage.setItem("forma.projects.v1", "not-json");
    await expect(listProjects()).rejects.toThrow("cannot be read");
    expect(localStorage.getItem("forma.projects.v1")).toBe("not-json");
  });
});
