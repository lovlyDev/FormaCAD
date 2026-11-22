import { useRef, useState } from "react";
import { saveFolderCopy } from "./folderCopy";
import type { ProjectSourceBinding } from "./sourceBinding";

export function useFolderCopy(
  binding: ProjectSourceBinding | null,
  reportSaved: (message: string) => void,
  reportError: (message: string) => void,
) {
  const active = useRef(false);
  const [copying, setCopying] = useState(false);

  async function save() {
    if (binding?.kind !== "folder" || active.current) return;
    active.current = true;
    setCopying(true);
    try {
      await saveFolderCopy(binding, reportSaved, reportError);
    } finally {
      active.current = false;
      setCopying(false);
    }
  }
  return { save, copying };
}
