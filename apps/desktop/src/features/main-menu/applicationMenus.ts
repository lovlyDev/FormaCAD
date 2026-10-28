import { t } from "../../i18n";
import type { MenuGroup } from "./MainMenu";
interface Actions {
  create: () => void;
  import: () => void;
  export: () => void;
  parameters: () => void;
  palette: () => void;
  settings: (page?: string) => void;
  workspaces: () => void;
}
export function applicationMenus(
  actions: Actions,
  projectOpen: boolean,
  blocked: boolean,
): MenuGroup[] {
  return [
    {
      id: "file",
      label: "File",
      actions: [
        {
          id: "new",
          label: t("New project"),
          shortcut: "Ctrl N",
          run: actions.create,
        },
        {
          id: "import",
          label: t("Import model"),
          shortcut: "Ctrl O",
          run: actions.import,
          disabled: blocked,
        },
        {
          id: "export",
          label: t("Export model"),
          shortcut: "Ctrl Shift E",
          run: actions.export,
          disabled: !projectOpen || blocked,
        },
      ],
    },
    {
      id: "edit",
      label: "Edit",
      actions: [
        {
          id: "parameters",
          label: t("Edit model parameters"),
          run: actions.parameters,
          disabled: !projectOpen || blocked,
        },
        {
          id: "search",
          label: t("Command palette"),
          shortcut: "F3",
          run: actions.palette,
        },
      ],
    },
    {
      id: "view",
      label: "View",
      actions: [
        {
          id: "appearance",
          label: t("Appearance"),
          run: () => actions.settings("Appearance"),
        },
        {
          id: "layout",
          label: t("Workspaces"),
          run: actions.workspaces,
          disabled: !projectOpen,
        },
      ],
    },
    {
      id: "add",
      label: "Add",
      actions: [
        {
          id: "feature",
          label: t("Edit model parameters"),
          run: actions.parameters,
          disabled: !projectOpen || blocked,
        },
        {
          id: "import",
          label: t("Import model"),
          run: actions.import,
          disabled: blocked,
        },
      ],
    },
    {
      id: "tools",
      label: "Tools",
      actions: [
        {
          id: "plugins",
          label: t("Plugins"),
          run: () => actions.settings("Plugins"),
        },
      ],
    },
    {
      id: "window",
      label: "Window",
      actions: [
        { id: "settings", label: t("Settings"), run: () => actions.settings() },
      ],
    },
    {
      id: "help",
      label: "Help",
      actions: [
        {
          id: "environment",
          label: t("CAD environment"),
          run: () => actions.settings("CAD environment"),
        },
      ],
    },
  ];
}
