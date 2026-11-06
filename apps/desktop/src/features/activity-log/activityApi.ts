import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

const event = z.object({
  id: z.number().int().positive(),
  kind: z.enum(["edit", "undo", "redo"]),
  toRevision: z.string().nullable(),
  createdAt: z.iso.datetime({ offset: true }),
});

export async function projectActivity(projectId: string) {
  return z.array(event).max(200).parse(await invoke("project_activity", { projectId }));
}
