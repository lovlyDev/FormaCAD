import { FileBox, FileImage, Plus, Download } from "lucide-react";
import { IconButton, TreeFolder } from "../../components/ui";
import { t } from "../../i18n";
import type { Project, ProjectFile, Revision } from "../../types";
export function ProjectFilesPanel({
  files,
  revision,
  blocked,
  onImport,
  onOpen,
  onEdit,
  exports,
  onExport,
}: {
  files: ProjectFile[];
  revision?: Revision;
  blocked: boolean;
  onImport: () => void;
  onOpen: (file: ProjectFile) => void;
  onEdit: () => void;
  exports: Project["exports"];
  onExport: (item: Project["exports"][number]) => void;
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
      <TreeFolder folderId="workspace" title={t("workspace")}>
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
      </TreeFolder>
      <TreeFolder
        folderId="attachments"
        title={t("attachments")}
        count={files.length}
      >
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
        {!files.length && (
          <p className="field-hint">{t("No attached files")}</p>
        )}
      </TreeFolder>
      <TreeFolder
        folderId="exports"
        title={t("exports")}
        count={exports.length}
      >
        {exports.map((item, index) => (
          <button
            key={`${item.name}:${index}`}
            className="tree-row file-row"
            onClick={() => onExport(item)}
          >
            <Download size={13} />
            <span className="truncate">{item.name}</span>
          </button>
        ))}
      </TreeFolder>
    </div>
  );
}
