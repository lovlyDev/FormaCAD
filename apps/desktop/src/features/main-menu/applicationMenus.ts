import { t } from "../../i18n";
import type { MenuGroup } from "./MainMenu";
import { datumLabels, type DatumKind } from "../add-menu/datum/datumModel";
interface Actions {
  duplicate: () => void;
  create: () => void;
  import: () => void;
  export: () => void;
  parameters: () => void;
  addTorus: () => void;
  addDatum: (kind: DatumKind) => void;
  shell?: () => void;
  pattern?: () => void;
  duplicateBody?: () => void;
  separateSolids?: () => void;
  booleanOperation?: () => void;
  splitBody?: () => void;
  thickenFace?: () => void;
  palette: () => void;
  settings: (page?: string) => void;
  workspaces: () => void;
}
export function applicationMenus(
  actions: Actions,
  projectOpen: boolean,
  blocked: boolean,
  canAddGeometry = false,
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
        {
          id: "duplicate",
          label: t("Duplicate project"),
          run: actions.duplicate,
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
          id: "torus",
          label: t("Torus"),
          run: actions.addTorus,
          disabled: blocked || !canAddGeometry,
        },
        ...(Object.keys(datumLabels) as DatumKind[]).map((kind) => ({
          id: kind,
          label: t(datumLabels[kind]),
          run: () => actions.addDatum(kind),
          disabled: blocked || !canAddGeometry,
        })),
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
          id: "shell",
          label: t("Shell"),
          disabled: !actions.shell,
          run: () => actions.shell?.(),
        },
        {
          id: "pattern",
          label: t("Pattern"),
          disabled: !actions.pattern,
          run: () => actions.pattern?.(),
        },
        {
          id: "duplicateBody",
          label: t("Duplicate body"),
          shortcut: "Shift D",
          disabled: !actions.duplicateBody,
          run: () => actions.duplicateBody?.(),
        },
        {
          id: "separateSolids",
          label: t("Separate solids"),
          shortcut: "P",
          disabled: !actions.separateSolids,
          run: () => actions.separateSolids?.(),
        },
        {
          id: "thickenFace",
          label: t("Thicken face"),
          disabled: !actions.thickenFace,
          run: () => actions.thickenFace?.(),
        },
        {
          id: "splitBody",
          label: t("Split body by plane"),
          disabled: !actions.splitBody,
          run: () => actions.splitBody?.(),
        },
        {
          id: "booleanOperation",
          label: t("Boolean operation"),
          disabled: !actions.booleanOperation,
          run: () => actions.booleanOperation?.(),
        },
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
