import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type ReactNode } from "react";
import { z } from "zod";
import { errorText, t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import type { Project } from "../../../types";
import type { ConfirmEdit } from "../../viewer/interaction/useViewportInteraction";
import { TorusDialog } from "./TorusDialog";

const definitionSchema = z.strictObject({
  expectedRevision: z.string().uuid(),
  expectedGeneration: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
  transactionId: z.string().uuid(),
  radii: z.strictObject({
    majorRadiusMm: z.number().positive().max(10000),
    minorRadiusMm: z.number().positive().max(10000),
  }).refine((radii) => radii.majorRadiusMm > radii.minorRadiusMm),
});

export function AdjustTorus({
  projectId, revisionId, close, onAdopt, confirmEdit, fallback,
}: {
  projectId: string;
  revisionId: string;
  close: () => void;
  onAdopt: (project: Project) => void;
  confirmEdit?: ConfirmEdit;
  fallback: ReactNode;
}) {
  const [definition, setDefinition] = useState<z.infer<typeof definitionSchema> | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  useEffect(() => {
    let active = true;
    void invoke("last_torus", { projectId }).then((value) => {
      const result = definitionSchema.safeParse(value);
      if (active && result.success && result.data.expectedRevision === revisionId)
        setDefinition(result.data);
    }).catch(() => {});
    return () => { active = false; };
  }, [projectId, revisionId]);
  if (!definition) return fallback;
  const apply = async (majorRadiusMm: number, minorRadiusMm: number) => {
    setError(undefined);
    const run = async () => {
      setBusy(true);
      try {
        const current = useWorkspace.getState().project;
        if (current?.id !== projectId || current.currentRevision !== revisionId)
          throw new Error("AMEND_STALE");
        const next = await invoke<Project>("amend_last_torus", {
          projectId,
          replacement: { ...definition, radii: { majorRadiusMm, minorRadiusMm } },
        });
        const active = useWorkspace.getState().project;
        if (next.id !== projectId) throw new Error("MODEL_APPLY_RESULT_INVALID");
        if (active?.id === projectId && active.currentRevision === revisionId)
          onAdopt(next);
        close();
        return true;
      } catch (cause) {
        setError(errorText(cause));
        return false;
      } finally {
        setBusy(false);
      }
    };
    await (confirmEdit
      ? confirmEdit(t("Adjust torus"), t("Replace the radii in the last modeling step."), t("Torus"), run)
      : run());
  };
  return <TorusDialog close={close} disabled={busy} initialRadii={definition.radii} error={error} onApply={apply} />;
}
