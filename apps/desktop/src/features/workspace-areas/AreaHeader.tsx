import { Columns2, Rows2, Maximize2, Minimize2, X } from "lucide-react";
import { Select, IconButton } from "../../components/ui";
import { t } from "../../i18n";
import { editorTypes, type Area, type EditorType } from "./layoutModel";
import { editorLabels } from "./editorLabels";
export function AreaHeader({
  area,
  maximized,
  canSplitX,
  canSplitY,
  canJoin,
  onEditor,
  onSplit,
  onJoin,
  onMaximize,
}: {
  area: Area;
  maximized: boolean;
  canSplitX: boolean;
  canSplitY: boolean;
  canJoin: boolean;
  onEditor: (editor: EditorType) => void;
  onSplit: (axis: "x" | "y") => void;
  onJoin: () => void;
  onMaximize: () => void;
}) {
  return (
    <header className="workspace-area-header">
      <Select
        aria-label={t("Editor type")}
        title={t(
          "An open editor moves here; the previous editor swaps places.",
        )}
        value={area.editor}
        onChange={(event) => onEditor(event.target.value as EditorType)}
      >
        {editorTypes.map((type) => (
          <option key={type} value={type}>
            {t(editorLabels[type])}
          </option>
        ))}
      </Select>
      <span className="workspace-area-actions">
        <IconButton
          label={t("Split area horizontally")}
          disabled={!canSplitX}
          onClick={() => onSplit("x")}
        >
          <Columns2 size={13} />
        </IconButton>
        <IconButton
          label={t("Split area vertically")}
          disabled={!canSplitY}
          onClick={() => onSplit("y")}
        >
          <Rows2 size={13} />
        </IconButton>
        <IconButton
          label={maximized ? t("Restore area") : t("Maximize area")}
          onClick={onMaximize}
        >
          {maximized ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
        </IconButton>
        <IconButton
          label={t("Join area with neighbor")}
          disabled={!canJoin}
          onClick={onJoin}
        >
          <X size={13} />
        </IconButton>
      </span>
    </header>
  );
}
