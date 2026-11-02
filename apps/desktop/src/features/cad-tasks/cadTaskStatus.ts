import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
export const cadTasksSchema = z.array(
  z.object({
    taskId: z.string(),
    projectId: z.string(),
    kind: z.string(),
    state: z.enum(["queued", "running"]),
  }),
);
export const readCadTasks = async () =>
  cadTasksSchema.parse(await invoke("cad_task_status"));
export const cancelCadTask = (taskId: string) =>
  invoke("cancel_cad_task", { taskId });
