import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "../../components/ui";
import { setLocale } from "../../i18n";
import { AccountButton } from "./AccountButton";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), openUrl: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke, isTauri: () => true }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: mocks.openUrl }));

const signedOut = { server: "https://cad.kartochk1n.ru", signedIn: false, profile: null, plan: null, subscriptionStatus: null, documentState: null, capabilities: [], offlineUntil: null, refreshAfter: null, syncedAt: null };
const signedIn = { ...signedOut, signedIn: true, profile: { sub: "u1", displayName: "Анна", avatarUrl: null, email: null, locale: "ru" }, plan: "pro", subscriptionStatus: "active", documentState: "valid", capabilities: ["support.priority"], offlineUntil: 1900000000, refreshAfter: 1900000000, syncedAt: 1800000000 };

const renderButton = () => render(<TooltipProvider><AccountButton /></TooltipProvider>);

describe("AccountButton", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.openUrl.mockReset();
    setLocale("ru");
  });

  it("signs in through the host and shows the profile without exposing tokens", async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => (cmd === "account_status" ? signedOut : signedIn));
    renderButton();
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("account_status"));
    fireEvent.click(screen.getByRole("button", { name: "Войти в аккаунт" }));
    fireEvent.click(await screen.findByRole("button", { name: "Войти через браузер" }));
    expect(await screen.findByText("Анна")).toBeTruthy();
    expect(mocks.invoke).toHaveBeenCalledWith("account_sign_in");
    expect(document.body.textContent).not.toMatch(/frt_|Bearer/);
  });

  it("offline refresh keeps cached rights and the CAD stays usable", async () => {
    const stale = { ...signedIn, documentState: "stale" };
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "account_refresh") throw JSON.stringify({ code: "ACCOUNT_OFFLINE", message: "connect error" });
      return stale;
    });
    renderButton();
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("account_refresh"));
    fireEvent.click(screen.getByRole("button", { name: "Анна" }));
    expect(await screen.findByText("Нужна проверка онлайн")).toBeTruthy();
    // The automatic background check is quiet when offline.
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("a revoked session is reported", async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "account_refresh") throw JSON.stringify({ code: "ACCOUNT_SESSION_ENDED", message: "revoked" });
      return { ...signedIn, documentState: "expired" };
    });
    renderButton();
    fireEvent.click(await screen.findByRole("button", { name: "Анна" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Сеанс аккаунта завершён");
  });
});
