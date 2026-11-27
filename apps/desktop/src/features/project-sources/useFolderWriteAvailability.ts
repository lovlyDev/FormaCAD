import { invoke } from "@tauri-apps/api/core";
import { skipToken, useQuery } from "@tanstack/react-query";
import { errorText, t } from "../../i18n";
import { native } from "../../lib/api";
import type { ProjectSourceBinding } from "./sourceBinding";

type Availability =
  | { status: "available" }
  | { status: "readOnly"; reason: string };

export function useFolderWriteAvailability(binding: ProjectSourceBinding | null) {
  const folder = binding?.kind === "folder" ? binding : null;
  const query = useQuery({
    queryKey: [
      "folder-write-availability", folder?.openingId, folder?.source.commitId,
    ],
    enabled: native && folder !== null,
    queryFn: folder ? () => invoke<Availability>("project_folder_write_availability", {
      projectId: folder.projectId,
      sourceId: folder.source.sourceId,
      expectedCommitId: folder.source.commitId,
    }) : skipToken,
    retry: false,
    refetchInterval: 3000,
    refetchOnWindowFocus: "always",
  });
  if (!folder) return { available: false, message: null, canCopy: false };
  if (query.isError) return { available: false, message: errorText(query.error), canCopy: false };
  if (query.data?.status === "readOnly") {
    return { available: false, message: errorText(new Error(query.data.reason)), canCopy: true };
  }
  return {
    available: query.data?.status === "available",
    message: query.isPending ? t("Checking project access…") : null,
    canCopy: false,
  };
}
