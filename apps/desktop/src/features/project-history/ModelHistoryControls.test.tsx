import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import * as Tooltip from "@radix-ui/react-tooltip";
import type { Project } from "../../types";
import { ModelHistoryControls } from "./ModelHistoryControls";

const status = vi.hoisted(() => vi.fn());
vi.mock("./historyApi", () => ({ modelHistoryStatus: status }));
vi.mock("../../lib/api", () => ({ native: true }));
const project = { id: "project", currentRevision: "rev", revisions: [{ id: "rev" }] } as Project;
afterEach(cleanup);
beforeEach(() => { status.mockReset(); });
function mount(blocked = false) {
  const action = vi.fn(); const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = render(<QueryClientProvider client={client}><Tooltip.Provider><ModelHistoryControls project={project} blocked={blocked} onAction={action} /></Tooltip.Provider></QueryClientProvider>);
  return { ...view, action };
}
it("runs only available model actions and respects keyboard ownership", async () => {
  status.mockResolvedValue({ canUndo: true, canRedo: false, expectedRevision: "rev" }); const view = mount();
  await waitFor(() => expect(view.getByRole("button", { name: "Undo model (Ctrl+Z)" })).toBeEnabled());
  expect(view.getByRole("button", { name: "Redo model (Ctrl+Y)" })).toBeDisabled();
  fireEvent.keyDown(window, { key: "z", ctrlKey: true }); expect(view.action).toHaveBeenCalledWith("undo");
  const input = document.createElement("input"); document.body.append(input);
  fireEvent.keyDown(input, { key: "z", ctrlKey: true }); input.remove(); expect(view.action).toHaveBeenCalledTimes(1);
});
it("does not use history status from an older model revision", async () => {
  status.mockResolvedValue({ canUndo: true, canRedo: true, expectedRevision: "old" }); const view = mount();
  await waitFor(() => expect(status).toHaveBeenCalled());
  expect(view.getByRole("button", { name: "Undo model (Ctrl+Z)" })).toBeDisabled();
  fireEvent.keyDown(window, { key: "z", ctrlKey: true }); expect(view.action).not.toHaveBeenCalled();
});
it("blocks both actions while a modal or model task owns the workspace", async () => {
  status.mockResolvedValue({ canUndo: true, canRedo: true, expectedRevision: "rev" }); const view = mount(true);
  await waitFor(() => expect(status).toHaveBeenCalled());
  fireEvent.keyDown(window, { key: "y", ctrlKey: true }); expect(view.action).not.toHaveBeenCalled();
  expect(view.getByRole("button", { name: "Redo model (Ctrl+Y)" })).toBeDisabled();
});
it("reports a failed history check and leaves model actions disabled", async () => {
  status.mockRejectedValue(new Error("broken history")); const view = mount();
  expect(await view.findByRole("status")).toHaveTextContent("Model history is unavailable");
  expect(view.getByRole("button", { name: "Undo model (Ctrl+Z)" })).toBeDisabled();
});
