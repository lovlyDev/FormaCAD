import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { t } from "../../i18n";
import { NewProjectDialog } from "./NewProjectDialog";

it("offers STEP geometry transfer with its limits and preserves normal creation", async () => {
  const create = vi.fn(async () => {}), transfer = vi.fn(async () => {});
  render(<NewProjectDialog open close={vi.fn()} onCreate={create} onTransferStep={transfer} />);
  expect(screen.getByText(t("STEP transfer creates a new project in millimeters with imported geometry. The original project and its files stay unchanged. Editing history is not transferred."))).toBeTruthy();
  fireEvent.change(screen.getByRole("textbox", { name: t("Project name") }), { target: { value: "Bracket" } });
  fireEvent.click(screen.getByRole("button", { name: t("Transfer STEP to folder") }));
  await waitFor(() => expect(transfer).toHaveBeenCalledExactlyOnceWith({ name: "Bracket", agent: "codex" }));
  expect(create).not.toHaveBeenCalled();
});
