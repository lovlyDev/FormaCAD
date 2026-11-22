import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TooltipProvider } from "@radix-ui/react-tooltip";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { t } from "../../i18n";
import { RecentFoldersPanel } from "./RecentFoldersPanel";

const mock = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", async original => ({
  ...await original<typeof import("@tauri-apps/api/core")>(),
  invoke: mock.invoke,
}));

const location = "E:/Projects/Bracket";
const items = [{ location, lastOpenedAtMs: 1, pinned: false }];
beforeEach(() => mock.invoke.mockReset());

function setup() {
  const open = vi.fn();
  const query = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = render(<QueryClientProvider client={query}>
    <TooltipProvider><RecentFoldersPanel openFolder={open} /></TooltipProvider>
  </QueryClientProvider>);
  return { open, ...view };
}

it("routes a row through the picker callback and pins only the index entry", async () => {
  mock.invoke.mockResolvedValueOnce({ items, rememberRecent: true });
  mock.invoke.mockResolvedValueOnce(true);
  mock.invoke.mockResolvedValueOnce({ items: [{ ...items[0], pinned: true }], rememberRecent: true });
  const { open } = setup();
  const row = await screen.findByRole("button", { name: location });
  expect(open).not.toHaveBeenCalled();
  fireEvent.click(row);
  expect(open).toHaveBeenCalledExactlyOnceWith(location);
  fireEvent.click(screen.getByRole("button", { name: t("Pin project folder") }));
  await screen.findByRole("button", { name: t("Unpin project folder") });
  expect(mock.invoke.mock.calls).toEqual([
    ["list_recent_folders"], ["pin_recent_folder", { location, pinned: true }], ["list_recent_folders"],
  ]);
});

it("saves the recording setting and refreshes it from the app index", async () => {
  mock.invoke.mockResolvedValueOnce({ items: [], rememberRecent: true });
  mock.invoke.mockResolvedValueOnce(undefined);
  mock.invoke.mockResolvedValueOnce({ items: [], rememberRecent: false });
  setup();
  const checkbox = await screen.findByRole("checkbox", { name: t("Remember opened folders") }) as HTMLInputElement;
  expect(checkbox.checked).toBe(true);
  fireEvent.click(checkbox);
  await waitFor(() => expect(checkbox.checked).toBe(false));
  expect(mock.invoke).toHaveBeenCalledWith("remember_recent_folders", { enabled: false });
});
