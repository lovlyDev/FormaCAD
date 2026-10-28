import { t } from "../../i18n";
import { editorTypes } from "../workspace-areas/layoutModel";
import { editorLabels } from "../workspace-areas/editorLabels";
import { showWorkspaceEditor } from "../workspace-areas/editorNavigation";
export function workspacePaletteCommands(projectId?: string) {
  return projectId
    ? editorTypes.map((editor) => ({
        label: t("Open editor {{value0}}", { value0: t(editorLabels[editor]) }),
        run: () => showWorkspaceEditor(projectId, editor),
      }))
    : [];
}
