import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type ReactNode } from "react";
import { z } from "zod";
import { errorText, t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import type { Project } from "../../../types";
import type { ConfirmEdit } from "../../viewer/interaction/useViewportInteraction";
import { DatumDialog } from "./DatumDialog";
import { datumKind, datumLabels, readDatumOperation, type DatumOperation } from "./datumModel";

const statusSchema = z.strictObject({
  expectedRevision: z.string().uuid(),
  expectedGeneration: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
  transactionId: z.string().uuid(),
  operation: z.unknown(),
});
type Definition = z.infer<typeof statusSchema> & { operation: DatumOperation };

export function AdjustDatum({ projectId, revisionId, close, onAdopt, confirmEdit, fallback }: {
  projectId: string;
  revisionId: string;
  close: () => void;
  onAdopt: (project: Project) => void;
  confirmEdit?: ConfirmEdit;
  fallback: ReactNode;
}) {
  const [definition, setDefinition] = useState<Definition | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  useEffect(() => {
    let active = true;
    void invoke("last_datum", { projectId }).then((value) => {
      const parsed = statusSchema.safeParse(value);
      if (!active || !parsed.success || parsed.data.expectedRevision !== revisionId) return;
      const operation = readDatumOperation(parsed.data.operation);
      if (operation) setDefinition({ ...parsed.data, operation });
    }).catch(() => {});
    return () => { active = false; };
  }, [projectId, revisionId]);
  if (!definition) return fallback;

  const apply = async (operation: DatumOperation) => {
    setError(undefined);
    const run = async () => {
      setBusy(true);
      try {
        const before = useWorkspace.getState().project;
        if (before?.id !== projectId || before.currentRevision !== revisionId)
          throw new Error("AMEND_STALE");
        const next = await invoke<Project>("amend_last_datum", {
          projectId, replacement: { ...definition, operation },
        });
        if (next.id !== projectId) throw new Error("MODEL_APPLY_RESULT_INVALID");
        const current = useWorkspace.getState().project;
        if (current?.id === projectId && current.currentRevision === revisionId) onAdopt(next);
        close();
        return true;
      } catch (cause) {
        setError(errorText(cause));
        return false;
      } finally {
        setBusy(false);
      }
    };
    if (confirmEdit) {
      await confirmEdit(t("Adjust last operation"), t("Adjust the last datum in the same Undo step."),
        t(datumLabels[datumKind(operation)]), run);
    } else {
      await run();
    }
  };
  return <DatumDialog kind={datumKind(definition.operation)} initial={definition.operation}
    close={close} disabled={busy} error={error} onApply={apply} />;
}
