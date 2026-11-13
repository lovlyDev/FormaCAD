import { useEffect, useMemo, useState } from "react";
import type { Project } from "../../types";
import { errorText } from "../../i18n";
import type { EdgeSelection } from "../viewer/edgeSelection";
import { captureMeasurement } from "../reference-measurements/measurementCapture";
import { measureReference } from "../reference-measurements/measurementApi";

export function useFilletEndpoints(project: Project, selection: readonly EdgeSelection[], disabled: boolean) {
  const captured = useMemo(() => {
    const first = selection[0];
    return selection.length === 1 && first?.topologyRef ? captureMeasurement({ project, bodyId: first.bodyId,
      query: { kind: "edgeEndpoints", reference: first.topologyRef }, selectionToken: JSON.stringify(first), interactive: !disabled }) : null;
  }, [project, selection, disabled]);
  const [state, setState] = useState<{ key: string; start?: number[]; end?: number[]; error?: string } | null>(null);
  useEffect(() => {
    if (!captured) return;
    let canceled = false;
    void measureReference(captured).then(report => {
      if (!canceled && report.result.kind === "edgeEndpoints")
        setState({ key: captured.key, start: report.result.startMm, end: report.result.endMm });
    }).catch(error => { if (!canceled) setState({ key: captured.key, error: errorText(error) }); });
    return () => { canceled = true; };
  }, [captured]);
  return state?.key === captured?.key ? state : null;
}
