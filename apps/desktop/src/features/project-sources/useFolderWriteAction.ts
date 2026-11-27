import { useRef, useState } from "react";
import { errorText } from "../../i18n";
import type { FolderWriteIntent } from "./permissionContract";
import { checkUncertainFolderWrite, isFolderWriteOpeningCurrent } from "./folderWriteResult";
import { runFolderWrite } from "./runFolderWrite";
import type { ProjectSourceBinding } from "./sourceBinding";
import type { FolderWriteResult } from "./writeApi";

export type FolderWriteOpening = Extract<ProjectSourceBinding, { kind: "folder" }>;
type Uncertain = {
  opening: FolderWriteOpening;
  result: Extract<FolderWriteResult, { status: "indeterminate" }>;
  observation?: "historical" | "absent";
};

/** One writer and one unresolved publication for every editing control in the workspace. */
export function useFolderWriteAction(
  binding: ProjectSourceBinding | null,
  onCommit: () => void,
  reportError: (message: string) => void,
  available = true,
) {
  const lock = useRef(false);
  const unresolved = useRef<Uncertain | null>(null);
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState<Uncertain | null>(null);
  const visible = binding?.kind === "folder"
    && uncertain?.opening.openingId === binding.openingId
    && uncertain.opening.source.commitId === binding.source.commitId
    ? uncertain : null;

  async function withLock(work: () => Promise<void>, opening: FolderWriteOpening) {
    lock.current = true;
    setBusy(true);
    try {
      await work();
    } catch (error) {
      if (isFolderWriteOpeningCurrent(opening)) reportError(errorText(error));
    } finally {
      lock.current = false;
      setBusy(false);
    }
  }

  async function write(
    opening: FolderWriteOpening,
    intent: FolderWriteIntent,
    detail: string,
    confirm: () => Promise<boolean>,
  ) {
    if (!available || lock.current || !isFolderWriteOpeningCurrent(opening)
      || (unresolved.current && isFolderWriteOpeningCurrent(unresolved.current.opening))) return;

    await withLock(async () => {
      const result = await runFolderWrite(opening, intent, detail, confirm);
      if (result.status === "committed") onCommit();
      else if (result.status === "rejectedBefore" && isFolderWriteOpeningCurrent(opening)) {
        reportError(errorText(new Error(result.error)));
      } else if (result.status === "indeterminate" && isFolderWriteOpeningCurrent(opening)) {
        unresolved.current = { opening, result };
        setUncertain(unresolved.current);
      }
    }, opening);
  }

  async function check() {
    if (lock.current || !visible || !isFolderWriteOpeningCurrent(visible.opening)) return;
    await withLock(async () => {
      const status = await checkUncertainFolderWrite(visible.opening, visible.result);
      if (status === "current") {
        unresolved.current = null;
        setUncertain(null);
        onCommit();
      } else if (status === "historical" || status === "absent") {
        unresolved.current = { ...visible, observation: status };
        setUncertain(unresolved.current);
      }
    }, visible.opening);
  }

  return { write, check, busy, uncertain: visible };
}

export type FolderWriteAction = ReturnType<typeof useFolderWriteAction>;
