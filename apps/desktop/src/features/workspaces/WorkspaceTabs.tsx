import { useState } from "react";
import { Plus, RotateCcw, Pencil } from "lucide-react";
import { t } from "../../i18n";
import { Button, IconButton } from "../../components/ui";
import type { SavedWorkspace } from "../../stores/layout/useWorkspaceLayout";
export function WorkspaceTabs({
  workspaces,
  active,
  onActivate,
  onCreate,
  onRename,
  onReset,
}: {
  workspaces: SavedWorkspace[];
  active: SavedWorkspace;
  onActivate: (id: string) => void;
  onCreate: (name: string) => void;
  onRename: (name: string) => void;
  onReset: () => void;
}) {
  const [editing, setEditing] = useState<"new" | "rename" | null>(null),
    [name, setName] = useState("");
  const title = (item: SavedWorkspace) =>
    item.name ||
    t(
      item.id === "model"
        ? "Model"
        : item.id === "inspect"
          ? "Inspect"
          : "Sketch",
    );
  return (
    <div className="workspace-presets">
      <div role="tablist" aria-label={t("Workspaces")}>
        {workspaces.map((item) => (
          <button
            role="tab"
            aria-selected={item.id === active.id}
            key={item.id}
            onClick={() => {
              onActivate(item.id);
              setEditing(null);
            }}
          >
            {title(item)}
          </button>
        ))}
      </div>
      <IconButton
        label={t("Create workspace")}
        disabled={workspaces.length >= 20}
        onClick={() => {
          setName("");
          setEditing("new");
        }}
      >
        <Plus size={14} />
      </IconButton>
      <IconButton
        label={t("Rename workspace")}
        onClick={() => {
          setName(title(active));
          setEditing("rename");
        }}
      >
        <Pencil size={13} />
      </IconButton>
      <IconButton label={t("Reset workspace layout")} onClick={onReset}>
        <RotateCcw size={13} />
      </IconButton>
      {editing && (
        <form
          className="workspace-name-form"
          onSubmit={(event) => {
            event.preventDefault();
            if (!name.trim()) return;
            (editing === "new" ? onCreate : onRename)(name);
            setEditing(null);
          }}
        >
          <input
            autoFocus
            aria-label={t("Workspace name")}
            maxLength={80}
            value={name}
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") setEditing(null);
            }}
          />
          <Button type="submit" disabled={!name.trim()}>
            {t("Save")}
          </Button>
          <Button type="button" onClick={() => setEditing(null)}>
            {t("Cancel")}
          </Button>
        </form>
      )}
    </div>
  );
}
