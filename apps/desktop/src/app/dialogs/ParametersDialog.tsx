import { useEffect, useState } from "react";
import { ArrowUpRight } from "lucide-react";
import { t, errorText } from "../../i18n";
import { Button, Modal, NumberInput } from "../../components/ui";
import { usePersistentState } from "../../lib/persistence";
import { parameterSchema } from "../../lib/model";
import { useWorkspace } from "../../stores/workspace";
import type { Parameters } from "../../types";
export function ParametersDialog({
  open,
  close,
  parameters,
  onApply,
  disabled,
}: {
  open: boolean;
  close: () => void;
  parameters: Parameters;
  onApply: (p: Parameters) => void;
  disabled: boolean;
}) {
  const draftKey = useWorkspace(s => `forma.ui.project.${s.project?.id ?? "home"}.parameters.${s.project?.currentRevision ?? "new"}`);
  const [draft, setDraft] = usePersistentState(draftKey, parameters);
  const [error, setError] = useState("");
  useEffect(() => {
    if (open) {
      setError("");
    }
  }, [open, parameters]);
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Model parameters")}
      description={t(
        "Dimensions are stored in millimeters. Changes create a new revision.",
      )}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const result = parameterSchema.safeParse(draft);
          if (!result.success) {
            setError(t("Invalid parameter value."));
            return;
          }
          onApply(result.data);
        }}
      >
        <div className="parameter-grid">
          {(
            [
              "width",
              "depth",
              "height",
              "thickness",
              "holeDiameter",
              "holes",
            ] as const
          ).map((key) => (
            <label key={key}>
              {
                {
                  width: t("Width"),
                  depth: t("Depth"),
                  height: t("Height"),
                  thickness: t("Wall thickness"),
                  holeDiameter: t("Hole diameter"),
                  holes: t("Number of holes"),
                }[key]
              }
              <div className="number-field">
                <NumberInput
                  disabled={disabled}
                  aria-label={t(key)}
                  type="number"
                  step={key === "holes" ? 1 : 0.1}
                  value={draft[key]}
                  onChange={(e) =>
                    setDraft({ ...draft, [key]: e.target.valueAsNumber })
                  }
                />
                <span>{key === "holes" ? "" : t("mm")}</span>
              </div>
            </label>
          ))}
        </div>
        {disabled && (
          <p className="field-hint">
            {t(
              "Imported meshes are inspected and exported as geometry. Parametric edits apply to native templates.",
            )}
          </p>
        )}
        {error && (
          <p className="error-inline" role="alert">
            {errorText(error)}
          </p>
        )}
        <div className="modal-actions">
          <Button type="button" onClick={close}>
            {t("Cancel")}
          </Button>
          <Button disabled={disabled} className="primary" type="submit">
            {t("Review changes")}
            <ArrowUpRight size={14} />
          </Button>
        </div>
      </form>
    </Modal>
  );
}
