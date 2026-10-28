import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "../../components/ui";
import { setLocale } from "../../i18n";
import type { PluginSummary } from "./api";
import { PluginToolbarMenu } from "./PluginToolbarMenu";
import { usePluginUi } from "./pluginUi";

const state = vi.hoisted(() => ({ plugins: [] as unknown[] }));
vi.mock("./usePluginCommands", async (original) => ({
  ...(await original<typeof import("./usePluginCommands")>()),
  usePlugins: () => ({ data: state.plugins }),
}));

const plugin = {
  manifest: {
    id: "turntable", name: "%name%", version: "1.0.0", description: "", categories: ["ui"],
    author: { name: "A", github: "a" }, repository: "https://github.com/a/b", license: "MIT",
    minAppVersion: "1.5.0", api: "0.2", permissions: ["viewport.capture"],
    contributes: { features: [], commands: [{ id: "record", title: "%command%", params: [] }], importers: [], exporters: [], panels: [], settings: [] },
    locales: ["ru"],
  },
  versions: [], active: "1.0.0", enabled: true, acceptedPermissions: ["viewport.capture"], settings: {},
  translations: { ru: { name: "Вращение", command: "Записать видео" } },
} as unknown as PluginSummary;

const view = () => render(<TooltipProvider><PluginToolbarMenu /></TooltipProvider>);

describe("plugin toolbar menu", () => {
  beforeEach(() => {
    setLocale("ru");
    usePluginUi.setState({ run: null });
  });

  it("is hidden while no enabled plugin has actions", () => {
    state.plugins = [{ ...plugin, enabled: false }];
    view();
    expect(screen.queryByRole("button", { name: "Действия плагинов" })).toBeNull();
  });

  it("gives an action with a known manifest icon its own button", () => {
    const withIcon = structuredClone(plugin);
    withIcon.manifest.contributes.commands[0].icon = "video";
    state.plugins = [withIcon];
    view();
    expect(screen.queryByRole("button", { name: "Действия плагинов" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Вращение: Записать видео" }));
    expect(usePluginUi.getState().run).toMatchObject({ kind: "command", contributionId: "record" });
  });

  it("lists actions without a known icon in the menu and opens the chosen one", () => {
    state.plugins = [plugin];
    view();
    fireEvent.click(screen.getByRole("button", { name: "Действия плагинов" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Вращение: Записать видео" }));
    expect(screen.queryByRole("menu")).toBeNull();
    expect(usePluginUi.getState().run).toMatchObject({ kind: "command", contributionId: "record" });
  });
});
