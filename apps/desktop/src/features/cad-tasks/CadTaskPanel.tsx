import { useQuery } from "@tanstack/react-query";
import { t, errorText } from "../../i18n";
import { native } from "../../lib/api";
import { Button } from "../../components/ui";
import { cancelCadTask, readCadTasks } from "./cadTaskStatus";
export function CadTaskPanel({ onError }: { onError: (text: string) => void }) {
  const status = useQuery({
    queryKey: ["cad-task-status"],
    queryFn: readCadTasks,
    enabled: native,
    refetchInterval: 750,
    retry: false,
  });
  return (
    <div className="workspace-content-list">
      {status.error && <p role="alert">{errorText(status.error)}</p>}
      {status.data?.map((task) => (
        <article key={task.taskId}>
          <strong>
            {task.state === "queued"
              ? t("CAD task queued")
              : t("CAD task running")}
          </strong>
          <p>{task.projectId}</p>
          <code>{task.kind}</code>
          <Button
            onClick={() =>
              void cancelCadTask(task.taskId)
                .then(() => status.refetch())
                .catch((error) => onError(errorText(error)))
            }
          >
            {t("Cancel CAD task")}
          </Button>
        </article>
      ))}
      {!status.data?.length && <p>{t("No active CAD task")}</p>}
    </div>
  );
}
