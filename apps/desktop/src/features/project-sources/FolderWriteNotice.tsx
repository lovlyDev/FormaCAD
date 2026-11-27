import { Copy, LockKeyhole } from "lucide-react";
import { Button } from "../../components/ui";
import { t } from "../../i18n";
import "../project-access/ProjectAccessNotice.css";

export function FolderWriteNotice({ message, canCopy, copying, copy }: {
  message: string | null;
  canCopy: boolean;
  copying: boolean;
  copy: () => void;
}) {
  if (!message) return null;
  return <div className="project-access-notice" role="status">
    <LockKeyhole size={16} />
    <span>{message}</span>
    {canCopy && <div>
      <Button disabled={copying} onClick={copy}>
        <Copy size={14} />{t("Save project copy to folder")}
      </Button>
    </div>}
  </div>;
}
