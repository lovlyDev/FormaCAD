import { useCallback, useEffect, useState } from "react";
import { native } from "../../lib/api";
import { errorText } from "../../i18n";
import { acquireAccess, retainAccess } from "./accessLifecycle";
import type { ProjectAccess } from "./projectAccessApi";

export function useProjectAccess(projectId: string | undefined, onError: (message: string) => void,scope=projectId) {
  const [result, setResult] = useState<{ id: string; scope:string|undefined; status?: ProjectAccess; error?: string } | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    if (!native || !projectId) return;
    let current = true;
    const release = retainAccess(projectId, cause => onError(errorText(cause)));
    void acquireAccess(projectId).then(status => { if (current) setResult({ id: projectId, scope,status }); })
      .catch(cause => { if (current) setResult({ id: projectId, scope,error: errorText(cause) }); });
    return () => { current = false; release(); };
  }, [projectId, scope,attempt, onError]);
  const retry = useCallback(() => { setResult(null); setAttempt(value => value + 1); }, []);
  const current = result && result.id === projectId && result.scope===scope ? result : null;
  const checking = native && !!projectId && !current;
  const writable = !native || !projectId || current?.status?.mode === "write";
  return { writable, checking, status: current?.status, error: current?.error, retry };
}
