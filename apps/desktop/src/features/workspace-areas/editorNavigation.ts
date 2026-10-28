import type { EditorType } from "./layoutModel";
export const editorNavigationEvent = "formacad:workspace-editor";
export interface EditorNavigation {
  projectId: string;
  editor: EditorType;
}
export function showWorkspaceEditor(projectId: string, editor: EditorType) {
  window.dispatchEvent(
    new CustomEvent<EditorNavigation>(editorNavigationEvent, {
      detail: { projectId, editor },
    }),
  );
}
