import { useQuery } from "@tanstack/react-query";
import { cancelCadTask, readCadTasks } from "./cadTaskStatus";
import { Clock3, LoaderCircle, X } from "lucide-react";
import { IconButton } from "../../components/ui";
import { native } from "../../lib/api";
import { t, errorText } from "../../i18n";
import "./CadTaskIndicator.css";
export function CadTaskIndicator({
  projectId,
  onError,
}: {
  projectId: string;
  onError: (error: string) => void;
}) {
  const status = useQuery({
    queryKey: ["cad-task-status"],
    queryFn: readCadTasks,
    enabled: native,
    refetchInterval: 750,
    retry: false,
  });
  const task = status.data?.find((task) => task.projectId === projectId);
  if (!task) return null;
  async function cancel() {
    try {
      await cancelCadTask(task!.taskId);
      await status.refetch();
    } catch (cause) {
      onError(errorText(cause));
    }
  }
  return (
    <div className="cad-task-indicator" role="status">
      {task.state === "queued" ? (
        <Clock3 size={14} />
      ) : (
        <LoaderCircle size={14} className="spin" />
      )}
      <span>
        {task.state === "queued" ? t("CAD task queued") : t("CAD task running")}
      </span>
      <IconButton label={t("Cancel CAD task")} onClick={() => void cancel()}>
        <X size={13} />
      </IconButton>
    </div>
  );
}
