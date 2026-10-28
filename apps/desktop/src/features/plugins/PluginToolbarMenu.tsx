import { useEffect, useRef, useState } from "react";
import { Puzzle } from "lucide-react";
import { IconButton } from "../../components/ui";
import { t } from "../../i18n";
import type { PluginSummary } from "./api";
import { pluginIcon } from "./pluginIcons";
import { usePluginUi } from "./pluginUi";
import { pluginText } from "./pluginText";
import { pluginPaletteCommands, usePlugins } from "./usePluginCommands";

/** Features and commands whose manifest icon is known get their own toolbar button. */
function iconActions(plugins: PluginSummary[]) {
  return plugins
    .filter((plugin) => plugin.enabled && !plugin.problem)
    .flatMap((plugin) => {
      const text = (value: string) => pluginText(plugin.translations, value);
      const c = plugin.manifest.contributes;
      return [
        ...c.features.map((item) => ({ kind: "feature" as const, item })),
        ...c.commands.map((item) => ({ kind: "command" as const, item })),
      ].flatMap(({ kind, item }) => {
        const Icon = pluginIcon(item.icon);
        if (!Icon) return [];
        return [{
          key: `${plugin.manifest.id}/${kind}/${item.id}`,
          label: t("{{value0}}: {{value1}}", { value0: text(plugin.manifest.name), value1: text(item.title) }),
          Icon,
          run: () => usePluginUi.getState().openRun({ plugin, kind, contributionId: item.id }),
        }];
      });
    });
}

/**
 * 3D-view toolbar entries for plugin actions: own buttons for actions with a
 * known icon, the rest in a "Plugin actions" menu (the same list as the
 * command palette). Nothing is shown while no enabled plugin has actions.
 */
export function PluginToolbarMenu() {
  const { data: plugins = [] } = usePlugins();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const buttons = iconActions(plugins);
  const direct = new Set(buttons.map((button) => button.label));
  const actions = pluginPaletteCommands(plugins, () => setOpen(false)).filter((action) => !direct.has(action.label));

  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);

  if (!buttons.length && !actions.length) return null;
  return (
    <>
      {buttons.map(({ key, label, Icon, run }) => (
        <IconButton key={key} label={label} onClick={run}>
          <Icon size={17} />
        </IconButton>
      ))}
      {actions.length > 0 && (
        <div className="plugin-toolbar-menu" ref={root}>
          <IconButton
            label={t("Plugin actions")}
            aria-haspopup="menu"
            aria-expanded={open}
            active={open}
            onClick={() => setOpen(!open)}
          >
            <Puzzle size={16} />
          </IconButton>
          {open && (
            <div className="plugin-toolbar-list" role="menu" aria-label={t("Plugin actions")}>
              {actions.map((action) => (
                <button key={action.label} type="button" role="menuitem" onClick={action.run}>
                  {action.label}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </>
  );
}
