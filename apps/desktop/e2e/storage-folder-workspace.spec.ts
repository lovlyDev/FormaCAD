import { expect, test } from "@playwright/test";

for (const locale of ["en", "ru"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`folder workspace keeps the selected source in ${locale}/${theme}`, async ({ page }) => {
      const id = "4614bd8b-53b6-4011-bc12-2b4e912aea67";
      const path = "E:/Projects/Folder fixture";
      await page.addInitScript(({ locale, theme, id, path }) => {
        localStorage.setItem("forma.locale", locale);
        const project = {
          id, schemaVersion: 1, name: "Folder fixture", units: "mm", agent: "codex",
          pinned: false, createdAt: "2026-11-20T00:00:00Z", updatedAt: "2026-11-20T00:00:00Z",
          currentRevision: null, revisions: [], files: [], exports: [], messages: [],
        };
        const managed = { ...project, name: "Managed fixture" };
        const calls: { command: string; args: unknown }[] = [];
        let recent = false;
        let next = 0;
        const selection = () => ({
          sourceId: `fixture-window:${++next}:nonce`, taskScopeId: `tasks-${next}`,
          commitId: "b827854d-c993-4654-a124-ae00fd759251", project,
        });
        Object.assign(window, {
          isTauri: true, folderFixtureCalls: calls,
          __TAURI_INTERNALS__: {
            metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
            transformCallback: () => 0,
            unregisterCallback: () => {},
            invoke: async (command: string, args: unknown) => {
              calls.push({ command, args });
              switch (command) {
                case "get_ui_preferences": return { "forma.theme": theme };
                case "list_projects": return { entries: [{ kind: "ready", rowKey: id, project: managed }], serializedProjectBytes: 1 };
                case "read_project_snapshot": return managed;
                case "create_project_folder": recent = true; return { status: "opened", source: selection() };
                case "open_project_folder": return selection();
                case "list_recent_folders": return { items: recent ? [{ location: path, lastOpenedAtMs: 1, pinned: false }] : [], rememberRecent: true };
                case "close_project_source": return true;
                case "project_folder_write_availability": return { status: "readOnly", reason: "PROJECT_READ_ONLY" };
                case "save_project_folder_copy": return null;
                case "history_status": return { canUndo: false, canRedo: false, expectedRevision: null };
                case "detect_environment": case "interrupted_sessions": case "permission_audit": return [];
                default: return null;
              }
            },
          },
        });
      }, { locale, theme, id, path });
      const labels = locale === "ru"
        ? { create: /^Создать проект/, folder: "Создать в папке", readOnly: /другом экземпляре/i, copy: "Сохранить копию проекта в папку" }
        : { create: /^Create a project/, folder: "Create in folder", readOnly: /another instance/i, copy: "Save project copy to folder" };
      await page.goto("/");
      await page.evaluate(theme => { document.documentElement.dataset.theme = theme; }, theme);
      await page.getByRole("button", { name: labels.create }).click();
      await page.getByRole("dialog").getByRole("button", { name: labels.folder, exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`#/folder/${id}$`));
      await expect(page.locator(".project-switch")).toContainText("Folder fixture");
      await expect(page.locator(".project-switch")).not.toContainText("Managed fixture");
      const readOnly = page.getByRole("status").filter({ hasText: labels.readOnly });
      await expect(readOnly).toBeVisible();
      await readOnly.getByRole("button", { name: labels.copy, exact: true }).click();
      await page.getByRole("button", { name: "forma", exact: true }).click();
      await expect(page).toHaveURL(/#\/$/);
      const calls = () => page.evaluate(() => (window as unknown as { folderFixtureCalls: { command: string; args: unknown }[] }).folderFixtureCalls);
      expect((await calls()).find(call => call.command === "save_project_folder_copy")?.args)
        .toEqual({ projectId: id, sourceId: "fixture-window:1:nonce", expectedCommitId: "b827854d-c993-4654-a124-ae00fd759251" });
      expect((await calls()).filter(call => call.command === "copy_project_for_editing")).toEqual([]);
      await expect.poll(async () => (await calls()).find(call => call.command === "close_project_source")?.args)
        .toEqual({ sourceId: "fixture-window:1:nonce" });
      await page.getByRole("button", { name: path, exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`#/folder/${id}$`));
      await expect(page.locator(".project-switch")).toContainText("Folder fixture");
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      expect((await calls()).filter(call => call.command === "open_project_folder").map(call => call.args))
        .toEqual([{ recentLocation: path }]);
      expect((await calls()).filter(call => call.command === "read_project_snapshot")).toEqual([]);
    });
  }
}

test("recovers a folder copy after a read-only open reports recovery", async ({ page }) => {
  const id = "13a3dfbd-9d7e-4e16-a97f-5159a9c2b974";
  await page.addInitScript(id => {
    localStorage.setItem("forma.locale", "en");
    const calls: { command: string; args: unknown }[] = [];
    let recoveries = 0;
    const project = {
      id, schemaVersion: 1, name: "Recovered fixture", units: "mm", agent: "codex",
      pinned: false, createdAt: "2026-11-20T00:00:00Z", updatedAt: "2026-11-20T00:00:00Z",
      currentRevision: null, revisions: [], files: [], exports: [], messages: [],
    };
    Object.assign(window, {
      isTauri: true, folderFixtureCalls: calls,
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
        transformCallback: () => 0,
        unregisterCallback: () => {},
        invoke: async (command: string, args: unknown) => {
          calls.push({ command, args });
          switch (command) {
            case "list_projects": return { entries: [], serializedProjectBytes: 0 };
            case "list_recent_folders": return { items: [], rememberRecent: false };
            case "open_project_folder": throw "PROJECT_SNAPSHOT_RECOVERY_REQUIRED";
            case "recover_project_folder_copy": return ++recoveries === 1 ? null : {
              status: "opened", source: { project, sourceId: "recovered:1", taskScopeId: "recovered-tasks", commitId: "recovered-commit" },
            };
            case "project_folder_write_availability": return { status: "available" };
            case "history_status": return { canUndo: false, canRedo: false, expectedRevision: null };
            case "detect_environment": case "interrupted_sessions": case "permission_audit": return [];
            default: return null;
          }
        },
      },
    });
  }, id);
  const calls = () => page.evaluate(() => (window as unknown as { folderFixtureCalls: { command: string; args: unknown }[] }).folderFixtureCalls);
  await page.goto("/");
  await page.getByRole("button", { name: "Open project in folder", exact: true }).click();
  await expect(page.getByText("This folder needs recovery. Use Recover a folder copy to keep the original files.", { exact: true })).toBeVisible();
  const recover = page.getByRole("button", { name: "Recover a folder copy", exact: true });
  await recover.click();
  await expect.poll(async () => (await calls()).filter(call => call.command === "recover_project_folder_copy").length).toBe(1);
  await expect(page).toHaveURL(new URL("/", page.url()).href);
  await recover.click();
  await expect(page).toHaveURL(new RegExp(`#/folder/${id}$`));
  await expect(page.locator(".project-switch")).toContainText("Recovered fixture");
  expect((await calls()).filter(call => call.command === "recover_project_folder_copy").map(call => call.args)).toEqual(
    Array.from({ length: 2 }, () => ({
      titles: { source: "Select the folder to recover", destination: "Select an empty folder for the recovered copy" },
    })),
  );
  expect((await calls()).filter(call => call.command === "read_project_snapshot")).toEqual([]);
});

test("offers STEP transfer from the dashboard after checking CAD availability on demand", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("forma.locale", "en");
    const calls: { command: string; args: unknown }[] = [];
    let transfers = 0;
    const project = {
      id: "step-import", schemaVersion: 1, name: "Transferred bracket", units: "mm",
      agent: "codex", pinned: false, createdAt: "2026-11-20T00:00:00Z", updatedAt: "2026-11-20T00:00:00Z",
      currentRevision: null, revisions: [], files: [], exports: [], messages: [],
    };
    Object.assign(window, {
      isTauri: true, folderFixtureCalls: calls,
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
        transformCallback: () => 0,
        unregisterCallback: () => {},
        invoke: async (command: string, args: unknown) => {
          calls.push({ command, args });
          switch (command) {
            case "get_ui_preferences": return {};
            case "list_projects": return { entries: [], serializedProjectBytes: 0 };
            case "list_recent_folders": return { items: [], rememberRecent: true };
            case "project_folder_write_availability": return { status: "available" };
            case "detect_environment": return [{ name: "OpenCascade kernel", available: true, detail: "Fixture" }];
            case "transfer_step_to_folder":
              if (++transfers === 1) return null;
              return { status: "opened", source: { sourceId: "fixture:1:nonce", taskScopeId: "step-task", commitId: "step-commit", project } };
            case "history_status": return { canUndo: false, canRedo: false, expectedRevision: null };
            case "interrupted_sessions": case "permission_audit": return [];
            default: return null;
          }
        },
      },
    });
  });
  await page.goto("/");
  const calls = () => page.evaluate(() => (window as unknown as { folderFixtureCalls: { command: string; args: unknown }[] }).folderFixtureCalls);
  const create = page.getByRole("button", { name: /^Create a project/ });
  await expect(create).toBeVisible();
  expect((await calls()).some(call => call.command === "detect_environment")).toBe(false);
  await create.click();
  const dialog = page.getByRole("dialog");
  const transfer = dialog.getByRole("button", { name: "Transfer STEP to folder", exact: true });
  await expect(transfer).toBeVisible();
  await dialog.getByRole("textbox", { name: "Project name" }).fill("Transferred bracket");
  await transfer.click();
  await expect(transfer).toBeEnabled();
  await expect(dialog).toBeVisible();
  await transfer.click();
  await expect(page).toHaveURL(/#\/folder\/step-import$/);
  await expect(page.locator(".project-switch")).toHaveText("Transferred bracket");
  expect((await calls()).filter(call => call.command === "detect_environment")).toHaveLength(1);
  const payload = { draft: { name: "Transferred bracket", agent: "codex", prompt: "Imported geometry from the selected STEP" } };
  expect((await calls()).filter(call => call.command === "transfer_step_to_folder").map(call => call.args))
    .toEqual([payload, payload]);
});

test("saves a draft then uses the reopened folder for Undo and Redo", async ({ page }) => {
  const projectId = "2cd42be8-acc3-4d24-934a-2f06f0736e2d";
  const now = "2026-11-20T00:00:00Z";
  const revision = "fixture-revision";
  const commits = [
    "e94bb321-c9fe-477c-88f6-e783d8d68a48",
    "5d8bf28e-94ab-45f7-89df-a83f53e57ec0",
    "11ed8482-d354-4853-9f65-7aca9a3083c0",
    "4a145be4-e889-4d9b-bf57-1b1b2c7ace50",
  ];
  const approvalId = "a8106158-4c6c-43a0-859c-6d8799330912";
  const program = JSON.stringify({
    version: 1,
    features: [
      { id: "Profile", operation: { type: "rectangle", width: 40, depth: 20 } },
      { id: "Pad", operation: { type: "extrude", sketch: "Profile", distance: 10 } },
    ],
    output: "Pad",
  });
  await page.addInitScript(({ projectId, now, revision, commits, approvalId, program }) => {
    localStorage.setItem("forma.locale", "en");
    const calls: { command: string; args: unknown }[] = [];
    let opening = 0;
    let head = 0;
    const sourceId = () => `folder-window:${opening}:nonce`;
    const project = () => ({
      id: projectId, schemaVersion: 1, name: "Editable folder", units: "mm", agent: "codex",
      pinned: false, createdAt: now, updatedAt: now, currentRevision: head % 2 ? revision : null,
      revisions: head ? [{
        id: revision, parent: null, createdAt: now, prompt: "Fixture revision", program,
        parameters: { kind: "box", width: 40, depth: 20, height: 10, thickness: 1, holeDiameter: 0, holes: 0 },
      }] : [], files: [], exports: [], messages: [],
    });
    Object.assign(window, {
      isTauri: true, folderFixtureCalls: calls,
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
        transformCallback: () => 0,
        unregisterCallback: () => {},
        invoke: async (command: string, args: unknown) => {
          calls.push({ command, args });
          switch (command) {
            case "get_ui_preferences": return {};
            case "list_projects": return { entries: [], serializedProjectBytes: 0 };
            case "list_recent_folders": return { items: [], rememberRecent: true };
            case "project_folder_write_availability": return { status: "available" };
            case "open_project_folder":
              opening += 1;
              return { sourceId: sourceId(), taskScopeId: `tasks-${opening}`, commitId: commits[head], project: project() };
            case "close_project_source": return true;
            case "detect_environment": return [{ name: "OpenCascade kernel", available: true, detail: "Fixture" }];
            case "request_permission": return { id: approvalId, requiresConfirmation: true };
            case "history_status": return { canUndo: head % 2 === 1, canRedo: head === 2, expectedRevision: project().currentRevision };
            case "apply_program": case "undo_model": case "redo_model": {
              const baseCommitId = commits[head++];
              return { status: "committed", sourceId: sourceId(), projectId, baseCommitId, commitId: commits[head], project: project() };
            }
            case "interrupted_sessions": case "permission_audit": return [];
            default: return null;
          }
        },
      },
    });
  }, { projectId, now, revision, commits, approvalId, program });
  await page.goto("/");
  const open = page.getByRole("button", { name: "Open project in folder", exact: true });
  const undo = page.getByRole("button", { name: "Undo model (Ctrl+Z)", exact: true });
  const redo = page.getByRole("button", { name: "Redo model (Ctrl+Y)", exact: true });
  const confirm = page.getByRole("button", { name: "Allow once", exact: true });

  await open.click();
  await expect(page.locator(".project-switch")).toHaveText("Editable folder");
  await page.getByRole("menuitem", { name: "Edit", exact: true }).click();
  await page.getByRole("menuitem", { name: "Edit model parameters", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "Model editor" });
  await editor.getByRole("textbox", { name: "CAD source" }).fill(program);
  await editor.getByRole("button", { name: "Build", exact: true }).click();
  await confirm.click();
  await expect(undo).toBeEnabled();

  await page.getByRole("button", { name: "forma", exact: true }).click();
  await open.click();
  await undo.click();
  await confirm.click();
  await redo.click();
  await confirm.click();
  await expect(undo).toBeEnabled();

  const calls = await page.evaluate(() =>
    (window as unknown as { folderFixtureCalls: { command: string; args: unknown }[] }).folderFixtureCalls,
  );
  expect(calls.find(call => call.command === "close_project_source")?.args)
    .toEqual({ sourceId: "folder-window:1:nonce" });
  const writes = [
    { command: "apply_program", sourceId: "folder-window:1:nonce", expectedCommitId: commits[0] },
    { command: "undo_model", sourceId: "folder-window:2:nonce", expectedCommitId: commits[1] },
    { command: "redo_model", sourceId: "folder-window:2:nonce", expectedCommitId: commits[2] },
  ];
  expect(calls.filter(call => writes.some(write => write.command === call.command))).toEqual(
    writes.map(({ command, sourceId, expectedCommitId }) => ({
      command, args: { projectId, folderWrite: { sourceId, expectedCommitId, approvalId } },
    })),
  );
  const permissions = calls.filter(call => call.command === "request_permission");
  expect(permissions.map(call => call.args)).toEqual([
    expect.objectContaining({ folderIntent: expect.objectContaining({ kind: "model", program, expectedRevision: null }) }),
    expect.objectContaining({ folderIntent: { kind: "history", direction: "undo", expectedRevision: revision } }),
    expect.objectContaining({ folderIntent: { kind: "history", direction: "redo", expectedRevision: null } }),
  ]);
  expect(calls.filter(call => call.command === "resolve_permission")).toHaveLength(3);
  expect(calls.some(call => call.command === "save_project" || call.command === "read_project_snapshot")).toBe(false);
});
