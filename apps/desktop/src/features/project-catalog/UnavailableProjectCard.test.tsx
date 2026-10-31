import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { setLocale, t, type Locale } from "../../i18n";
import { UnavailableProjectCard } from "./UnavailableProjectCard";
import type { UnavailableEntry } from "./useProjectCatalog";
afterEach(() => { cleanup(); setLocale("ru"); document.documentElement.removeAttribute("data-theme"); });
it.each(["ru", "en"] as Locale[])("shows an unavailable row without model actions in %s and both themes", locale => {
  setLocale(locale);
  const entry: UnavailableEntry = { kind: "unavailable", rowKey: "managed-index:9007199254740993", index: { projectId: null, idHint: "invalid", name: null, updatedAt: null }, issue: { status: "recovery_required", code: "PROJECT_SNAPSHOT_RECOVERY_REQUIRED", details: "raw technical details" } };
  const retry = vi.fn();
  for (const theme of ["light", "dark"]) {
    document.documentElement.setAttribute("data-theme", theme);
    const view = render(<UnavailableProjectCard entry={entry} onRetry={retry} />);
    expect(screen.getByRole("article", { name: t("Unavailable project") }).getAttribute("data-row-key")).toBe(entry.rowKey);
    expect(screen.getByText(t("This project requires recovery. No automatic repair was performed."))).toBeTruthy();
    expect(screen.getAllByRole("button")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: t("Refresh project catalog") }));
    expect(screen.getByText(t("Technical details"))).toBeTruthy();
    view.unmount();
  }
  expect(retry).toHaveBeenCalledTimes(2);
});
