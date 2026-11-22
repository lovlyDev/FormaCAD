import { expect, it, vi } from "vitest";
import { applicationMenus } from "./applicationMenus";

const actions = {
  duplicate: vi.fn(), create: vi.fn(), import: vi.fn(), export: vi.fn(),
  parameters: vi.fn(), palette: vi.fn(), settings: vi.fn(), workspaces: vi.fn(),
};

it("offers a folder copy and source editor while managed writes remain blocked", () => {
  const saveCopy = vi.fn();
  const menus = applicationMenus(actions, true, true, {
    editProgramBlocked: false, copyBlocked: false, saveCopy,
  });
  const action = (menuId: string, id: string) => menus.find(group => group.id === menuId)?.actions.find(item => item.id === id);
  const copy = action("file", "folder-copy");
  expect(copy?.disabled).toBe(false);
  copy?.run();
  expect(saveCopy).toHaveBeenCalledOnce();
  expect(action("file", "duplicate")).toBeUndefined();
  expect(action("file", "import")?.disabled).toBe(true);
  expect(action("edit", "parameters")?.disabled).toBe(false);
});

it("keeps the existing managed file actions", () => {
  const groups = applicationMenus(actions, true, false);
  const file = groups.find(group => group.id === "file")?.actions;
  expect(file?.find(item => item.id === "duplicate")?.run).toBe(actions.duplicate);
  expect(file?.some(item => item.id === "folder-copy")).toBe(false);
});
