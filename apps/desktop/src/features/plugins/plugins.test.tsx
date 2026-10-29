import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "../../components/ui";
import { setLocale } from "../../i18n";
import type { FormField, InstallPreview, PluginManifest } from "./api";
import { PluginForm } from "./PluginForm";
import { PluginInstallDialog } from "./PluginInstallDialog";
import { usePluginUi } from "./pluginUi";
import { fieldError, initialValues, permissionText, pluginText } from "./pluginText";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke, isTauri: () => true }));

const fields: FormField[] = [
  { key: "teeth", type: "integer", label: "%teeth%", default: 16, min: 6, max: 48 },
  { key: "width", type: "length", label: "Width", default: 3 },
  { key: "mode", type: "choice", label: "Mode", options: [{ value: "a", label: "A" }, { value: "b", label: "B" }] },
];
const translations = { ru: { teeth: "Число зубьев", name: "Шестерня" }, en: { teeth: "Teeth", name: "Gear" } };

const manifest: PluginManifest = {
  id: "simple-gear", name: "%name%", version: "0.2.0", description: "Gear", categories: ["modeling"],
  author: { name: "FormaCAD", github: "lovlyDev" }, repository: "https://github.com/lovlyDev/FormaCAD", license: "MIT",
  minAppVersion: "1.3.3", api: "0.1", permissions: ["model.write", "network:api.example.com"],
  contributes: { features: [], commands: [], importers: [], exporters: [], panels: [], settings: [] }, locales: ["en", "ru"],
};
const preview: InstallPreview = {
  token: "token-1", manifest, permissions: ["model.write", "network:api.example.com"], addedPermissions: ["network:api.example.com"],
  installedVersion: "0.1.0", isUpdate: true, isDowngrade: false, alreadyInstalled: false, source: { kind: "file" },
  sha256: "ab".repeat(32), translations,
};

describe("plugin texts and forms", () => {
  beforeEach(() => setLocale("ru"));

  it("resolves %key% strings from plugin locales and never from the app catalog", () => {
    expect(pluginText(translations, "%teeth%")).toBe("Число зубьев");
    setLocale("en");
    expect(pluginText(translations, "%teeth%")).toBe("Teeth");
    expect(pluginText(translations, "Settings")).toBe("Settings");
    expect(permissionText("network:api.example.com")).toContain("api.example.com");
  });

  it("fills defaults and validates like the host", () => {
    const values = initialValues(fields);
    expect(values).toEqual({ teeth: 16, width: 3, mode: "a" });
    expect(fieldError(fields[0], 5)).not.toBeNull();
    expect(fieldError(fields[0], 6.5)).not.toBeNull();
    expect(fieldError(fields[2], "c")).not.toBeNull();
    expect(fieldError(fields[0], 20)).toBeNull();
  });

  it("renders the declarative form with FormaCAD controls", () => {
    const onChange = vi.fn();
    render(<TooltipProvider><PluginForm fields={fields} values={initialValues(fields)} onChange={onChange} translations={translations} /></TooltipProvider>);
    const teeth = screen.getByLabelText("Число зубьев");
    fireEvent.change(teeth, { target: { value: "20" } });
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ teeth: 20 }));
    expect(screen.getByText("мм")).toBeTruthy();
  });
});

describe("PluginInstallDialog", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    setLocale("ru");
    usePluginUi.getState().showPreview(preview);
  });

  const renderDialog = (onInstalled = vi.fn()) =>
    render(
      <QueryClientProvider client={new QueryClient()}>
        <TooltipProvider><PluginInstallDialog onInstalled={onInstalled} /></TooltipProvider>
      </QueryClientProvider>,
    );

  it("shows new permissions of an update and installs only after confirmation", async () => {
    mocks.invoke.mockResolvedValue({});
    const onInstalled = vi.fn();
    renderDialog(onInstalled);
    expect(screen.getByText("Обновление плагина")).toBeTruthy();
    expect(screen.getByText("Шестерня")).toBeTruthy();
    expect(screen.getByText("Новое")).toBeTruthy();
    expect(screen.getByText(/Эта версия просит новые права/)).toBeTruthy();
    expect(mocks.invoke).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Обновить" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("plugins_install_confirm", { token: "token-1" }));
    expect(onInstalled).toHaveBeenCalled();
  });

  it("cancel discards the pending install", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    renderDialog();
    fireEvent.click(screen.getByRole("button", { name: "Отмена" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("plugins_install_cancel", { token: "token-1" }));
    expect(usePluginUi.getState().preview).toBeNull();
  });
});
