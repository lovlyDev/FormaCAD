import { FileBox, FileImage, Plus } from "lucide-react";
import { IconButton } from "../../components/ui";
import { t } from "../../i18n";
import type { ProjectFile, Revision } from "../../types";
export function ProjectFilesPanel({
  files,
  revision,
  blocked,
  onImport,
  onOpen,
  onEdit,
}: {
  files: ProjectFile[];
  revision?: Revision;
  blocked: boolean;
  onImport: () => void;
  onOpen: (file: ProjectFile) => void;
  onEdit: () => void;
}) {
  return (
    <div className="workspace-content-list">
      <div className="panel-heading">
        <span>{t("PROJECT FILES")}</span>
        <IconButton
          label={t("Import files")}
          disabled={blocked}
          onClick={onImport}
        >
          <Plus size={14} />
        </IconButton>
      </div>
      {revision && (
        <button className="tree-row file-row" onClick={onEdit}>
          <FileBox size={14} />
          {revision.program
            ? revision.program.trimStart().startsWith("{")
              ? "model.cad.json"
              : "model.py"
            : "model.parameters.json"}
        </button>
      )}
      {files.map((file) => (
        <button
          className="tree-row file-row"
          key={file.name}
          onClick={() => onOpen(file)}
        >
          {file.kind === "image" ? (
            <FileImage size={14} />
          ) : (
            <FileBox size={14} />
          )}
          <span className="truncate">{file.name}</span>
        </button>
      ))}
      {!files.length && <p className="field-hint">{t("No attached files")}</p>}
    </div>
  );
}
