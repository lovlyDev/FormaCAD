import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { Group } from "three";
import { newProject } from "../../../stores/workspace";
import { defaults } from "../../../types";
import { useSavedBaseline } from "./useSavedBaseline";
const mocks = vi.hoisted(() => ({ load: vi.fn(), dispose: vi.fn() }));
vi.mock("../../../lib/files", () => ({ loadModel: mocks.load }));
vi.mock("../../../lib/model", () => ({ disposeModel: mocks.dispose }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
function base() {
  const project = newProject("Review", "blank", "mm", "codex"); project.currentRevision = "base";
  project.revisions = [{ id: "base", parent: null, createdAt: project.createdAt, prompt: "", parameters: defaults, preview: "base.glb" }];
  project.files = [{ name: "base.glb", kind: "model", size: 100 }]; return project;
}
it("disposes a late loaded baseline when the review closes", async () => {
  let resolve!: (object: Group) => void; mocks.load.mockImplementation(() => new Promise<Group>(done => { resolve = done; }));
  const project = base(); const mounted = renderHook(() => useSavedBaseline(project)); mounted.unmount();
  const late = new Group(); await act(async () => resolve(late)); expect(mocks.dispose).toHaveBeenCalledWith(late);
});
it("owns and disposes the loaded copy without modifying project file metadata", async () => {
  const project = base(), original = JSON.stringify(project), object = new Group(); mocks.load.mockResolvedValue(object);
  const mounted = renderHook(() => useSavedBaseline(project)); await act(async () => {});
  expect(mounted.result.current.object).toBe(object); expect(JSON.stringify(project)).toBe(original);
  mounted.unmount(); expect(mocks.dispose).toHaveBeenCalledWith(object);
});
