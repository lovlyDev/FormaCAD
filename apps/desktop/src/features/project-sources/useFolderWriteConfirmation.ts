import { useEffect, useRef, useState } from "react";
import type { ActionReviewContent } from "../../app/dialogs/ActionReviewDialog";
import { isFolderWriteOpeningCurrent } from "./folderWriteResult";
import type { ProjectSourceBinding } from "./sourceBinding";
import type { FolderWriteOpening } from "./useFolderWriteAction";

export function useFolderWriteConfirmation(binding: ProjectSourceBinding | null) {
  const decision = useRef<{ opening: FolderWriteOpening; resolve: (allow: boolean) => void } | null>(null);
  const [pending, setPending] = useState<ActionReviewContent | null>(null);

  useEffect(() => {
    const active = decision.current;
    if (active && !isFolderWriteOpeningCurrent(active.opening)) {
      decision.current = null;
      active.resolve(false);
    }
  }, [binding]);

  useEffect(() => () => {
    decision.current?.resolve(false);
    decision.current = null;
  }, []);

  async function confirm(opening: FolderWriteOpening, content: ActionReviewContent) {
    if (!isFolderWriteOpeningCurrent(opening) || decision.current) {
      return false;
    }
    return new Promise<boolean>((resolve) => {
      decision.current = { opening, resolve };
      setPending(content);
    });
  }

  function decide(allow: boolean) {
    const active = decision.current;
    decision.current = null;
    setPending(null);
    active?.resolve(allow && isFolderWriteOpeningCurrent(active.opening));
  }

  const active = decision.current;
  return {
    confirm,
    decide,
    pending: active && binding === active.opening ? pending : null,
  };
}
