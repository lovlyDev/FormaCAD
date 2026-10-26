import { useQuery } from "@tanstack/react-query";
import { t } from "../../i18n";
import { native } from "../../lib/api";
import { listPlugins, type PluginSummary } from "./api";
import { usePluginUi, type RunKind } from "./pluginUi";
import { pluginText } from "./pluginText";

export interface PaletteCommand {
  label: string;
  run: () => void;
}

export function usePlugins(enabled = true) {
  return useQuery({ queryKey: ["plugins"], queryFn: listPlugins, enabled: native && enabled, staleTime: 30000 });
}

/** Command palette entries for every contribution of enabled plugins. */
export function pluginPaletteCommands(plugins: PluginSummary[], close: () => void): PaletteCommand[] {
  const { openRun, openPanel } = usePluginUi.getState();
  return plugins
    .filter((plugin) => plugin.enabled && !plugin.problem)
    .flatMap((plugin) => {
      const text = (value: string) => pluginText(plugin.translations, value);
      const name = text(plugin.manifest.name);
      const c = plugin.manifest.contributes;
      const action = (kind: RunKind, id: string, title: string, template: string) => ({
        label: t(template, { value0: name, value1: text(title) }),
        run: () => {
          close();
          openRun({ plugin, kind, contributionId: id });
        },
      });
      return [
        ...c.features.map((item) => action("feature", item.id, item.title, "{{value0}}: {{value1}}")),
        ...c.commands.map((item) => action("command", item.id, item.title, "{{value0}}: {{value1}}")),
        ...c.importers.map((item) => action("import", item.id, item.title, "{{value0}}: import {{value1}}")),
        ...c.exporters.map((item) => action("export", item.id, item.title, "{{value0}}: export {{value1}}")),
        ...c.panels.map((item) => ({
          label: t("{{value0}}: panel {{value1}}", { value0: name, value1: text(item.title) }),
          run: () => {
            close();
            openPanel({ plugin, panelId: item.id });
          },
        })),
      ];
    });
}
