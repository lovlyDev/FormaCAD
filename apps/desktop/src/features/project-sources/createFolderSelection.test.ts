import { beforeEach, expect, it, vi } from "vitest";
import { t } from "../../i18n";
import { createFolderSelection, type BlankFolderDraft } from "./createFolderSelection";
import { transferStepToFolder } from "./stepFolderSelection";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", async original => ({
  ...await original<typeof import("@tauri-apps/api/core")>(),
  invoke: mocks.invoke,
  isTauri: () => true,
}));

const draft: BlankFolderDraft = { name: "A new part", units: "mm", agent: "codex" };
const project = { id: "new-project" };
const source = { sourceId: "window:1:nonce", commitId: "written", taskScopeId: "task" };

beforeEach(() => { mocks.invoke.mockReset(); });

function setup() {
  return { target: { isCurrent: vi.fn(() => true), activate: vi.fn() }, report: vi.fn() };
}

it("sends only draft metadata to the native picker and adopts the returned project", async () => {
  const { target, report } = setup();
  mocks.invoke.mockResolvedValueOnce({ status: "opened", source: { project, ...source } });
  expect(await createFolderSelection(draft, target, report)).toBe(true);
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("create_project_folder", { draft });
  expect(target.activate).toHaveBeenCalledExactlyOnceWith(project, source);
  expect(report).not.toHaveBeenCalled();
});

it("leaves the dialog open after the native picker is cancelled", async () => {
  const { target, report } = setup();
  mocks.invoke.mockResolvedValueOnce(null);
  expect(await createFolderSelection(draft, target, report)).toBe(false);
  expect(target.activate).not.toHaveBeenCalled();
  expect(report).not.toHaveBeenCalled();
});

it.each([
  ["savedWithoutOpening", "The project was saved. Open the selected folder to continue."],
  ["indeterminate", "The save result is uncertain. Open the selected folder to check it."],
] as const)("reports %s and asks the user to open the folder without another write", async (status, label) => {
  const { target, report } = setup();
  mocks.invoke.mockResolvedValueOnce({ status, commitId: "attempt", error: "Disk failure" });
  expect(await createFolderSelection(draft, target, report)).toBe(true);
  expect(report).toHaveBeenCalledExactlyOnceWith(`${t(label)}\nattempt\nDisk failure`);
  expect(target.activate).not.toHaveBeenCalled();
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});

it("releases a new live source from a late creation without switching view", async () => {
  const { target, report } = setup();
  mocks.invoke.mockResolvedValueOnce({ status: "opened", source: { project, ...source } });
  const pending = createFolderSelection(draft, target, report);
  target.isCurrent.mockReturnValue(false);
  expect(await pending).toBe(false);
  expect(mocks.invoke).toHaveBeenLastCalledWith("close_project_source", { sourceId: source.sourceId });
  expect(target.activate).not.toHaveBeenCalled();
  expect(report).not.toHaveBeenCalled();
});

it("starts the STEP transfer without passing native paths from the renderer", async () => {
  const { target, report } = setup();
  mocks.invoke.mockResolvedValueOnce({ status: "opened", source: { project, ...source } });
  expect(await transferStepToFolder(draft, target, report)).toBe(true);
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("transfer_step_to_folder", {
    draft: { name: draft.name, agent: draft.agent, prompt: t("Imported geometry from the selected STEP") },
  });
  expect(target.activate).toHaveBeenCalledExactlyOnceWith(project, source);
});

it("keeps the workspace when either native STEP picker is cancelled", async () => {
  const { target, report } = setup();
  mocks.invoke.mockResolvedValueOnce(null);
  expect(await transferStepToFolder(draft, target, report)).toBe(false);
  expect(target.activate).not.toHaveBeenCalled();
  expect(report).not.toHaveBeenCalled();
});
