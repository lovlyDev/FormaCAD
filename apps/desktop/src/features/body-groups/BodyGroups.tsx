import { useState } from "react";
import { Folder, Plus, Minus, Trash2 } from "lucide-react";
import { Button, IconButton } from "../../components/ui";
import { t } from "../../i18n";
import { useWorkspace } from "../../stores/workspace";
import type { BodyGroup, GroupCommand } from "./bodyGroupCommands";
import "./bodyGroups.css";

export function BodyGroups({ groups, selectable, onCommand }: {
  groups: BodyGroup[];
  selectable: string[];
  onCommand?: (command: GroupCommand) => void;
}) {
  const selectedBodies = useWorkspace((state) => state.selectedBodies);
  const select = useWorkspace((state) => state.setBodySelection);
  const [name, setName] = useState("");
  const label = name.trim();
  const valid = !!label && Array.from(label).length <= 120 && !/\p{Cc}/u.test(name);
  const selected = selectedBodies.filter((id) => selectable.includes(id));
  return (
    <section aria-label={t("Body groups")} className="body-groups">
      <form onSubmit={(event) => {
        event.preventDefault();
        if (!onCommand || !valid) return;
        onCommand({ command: "group_create", group: { id: crypto.randomUUID(), name: label, bodyIds: selected } });
      }}>
        <input aria-label={t("New body group")} placeholder={t("New body group")}
          value={name} disabled={!onCommand} onChange={(event) => setName(event.target.value)} />
        <Button type="submit" disabled={!onCommand || !valid}>{t("Create group")}</Button>
      </form>
      {groups.map((group) => {
        const members = group.bodyIds.filter((id) => selectable.includes(id));
        const add = selected.filter((id) => !group.bodyIds.includes(id));
        const remove = selected.filter((id) => group.bodyIds.includes(id));
        return (
          <div key={group.id} className="body-group-row">
            <Button disabled={!members.length} onClick={() => select(members, members[0])}>
              <Folder size={14} aria-hidden />{group.name}
            </Button>
            <IconButton label={t("Add selection to {{name}}", { name: group.name })}
              disabled={!onCommand || !add.length}
              onClick={() => onCommand?.({ command: "group_add", groupId: group.id, bodyIds: add })}>
              <Plus size={14} />
            </IconButton>
            <IconButton label={t("Remove selection from {{name}}", { name: group.name })}
              disabled={!onCommand || !remove.length}
              onClick={() => onCommand?.({ command: "group_remove", groupId: group.id, bodyIds: remove })}>
              <Minus size={14} />
            </IconButton>
            <IconButton label={t("Delete group {{name}}", { name: group.name })}
              disabled={!onCommand}
              onClick={() => onCommand?.({ command: "group_remove", groupId: group.id })}>
              <Trash2 size={14} />
            </IconButton>
          </div>
        );
      })}
    </section>
  );
}
