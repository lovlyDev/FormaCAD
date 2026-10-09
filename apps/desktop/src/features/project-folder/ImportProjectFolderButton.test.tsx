import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ImportProjectFolderButton } from "./ImportProjectFolderButton";
const api = vi.hoisted(() => ({ inspect: vi.fn(), import: vi.fn() }));
vi.mock("./projectFolderApi", () => ({ inspectProjectFolder: api.inspect, importProjectFolder: api.import }));
const folder = { path: "C:\\Projects\\part", projectId: "project", name: "Checked part", currentRevision: "rev", revisionCount: 3, manifestSha256: "a".repeat(64), identityExists: true };
beforeEach(() => { api.inspect.mockReset(); api.import.mockReset(); });
afterEach(cleanup);
it("requires review and copies conflicting identity without importing on cancel", async () => {
  api.inspect.mockResolvedValue(folder); const imported = vi.fn();
  const view = render(<ImportProjectFolderButton onImported={imported} onError={vi.fn()} />);
  fireEvent.click(view.getByRole("button", { name: "Import project folder" }));
  expect(await view.findByRole("dialog")).toHaveTextContent("Checked part");
  expect(view.getByRole("checkbox")).toBeChecked(); expect(view.getByRole("checkbox")).toBeDisabled();
  expect(api.import).not.toHaveBeenCalled(); fireEvent.click(view.getByRole("button", { name: "Cancel" }));
  expect(imported).not.toHaveBeenCalled(); expect(api.import).not.toHaveBeenCalled();
});
it("passes the captured manifest and preserves review after a changed source rejection", async () => {
  api.inspect.mockResolvedValue(folder); api.import.mockRejectedValue(new Error("PROJECT_FOLDER_CHANGED"));
  const view = render(<ImportProjectFolderButton onImported={vi.fn()} onError={vi.fn()} />);
  fireEvent.click(view.getByRole("button", { name: "Import project folder" })); await view.findByRole("dialog");
  fireEvent.click(view.getByRole("button", { name: "Import project" }));
  await waitFor(() => expect(api.import).toHaveBeenCalledWith(folder, true));
  expect(await view.findByRole("alert")).toHaveTextContent("The project folder changed after review. Select it again.");
  expect(view.getByRole("dialog")).toBeVisible();
});
