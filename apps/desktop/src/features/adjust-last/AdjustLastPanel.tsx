import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { Button, IconButton } from "../../components/ui";
import { errorText, t } from "../../i18n";
import { useWorkspace } from "../../stores/workspace";
import type { Project } from "../../types";
import { useTransformViewport } from "../transform/viewport/transformStore";
import type { Axis } from "../transform/frames";
import { fieldsFromSession, type AdjustFields } from "./adjustFields";
import { amendLast } from "./amendLast";
import { AdjustTorus } from "../add-menu/torus/AdjustTorus";
import { AdjustDatum } from "../add-menu/datum/AdjustDatum";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { readDatumOperation } from "../add-menu/datum/datumModel";
import type { ConfirmEdit } from "../viewer/interaction/useViewportInteraction";
import "./adjustLast.css";

const axes: Axis[] = ["x", "y", "z"];
const kindTitle = { move: "Move", rotate: "Rotate", scale: "Scale" } as const;

function NumberField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(Number(value.toPrecision(12)))), [value]);
  return (
    <label className="adjust-last-field">
      <span>{label}</span>
      <input
        inputMode="decimal"
        value={text}
        onChange={(event) => {
          setText(event.target.value);
          const parsed = Number(event.target.value.replace(",", "."));
          if (event.target.value.trim() && Number.isFinite(parsed))
            onChange(parsed);
        }}
      />
    </label>
  );
}

/** F9 panel: parameters of the latest own transform, replaced through the host amend route. */
export function AdjustLastPanel({
  onAdopt,
  confirmEdit,
}: {
  onAdopt: (project: Project) => void;
  confirmEdit?: ConfirmEdit;
}) {
  const open = useTransformViewport((state) => state.adjustOpen);
  const record = useTransformViewport((state) => state.last);
  const setOpen = useTransformViewport((state) => state.setAdjustOpen);
  const setLast = useTransformViewport((state) => state.setLast);
  const project = useWorkspace((state) => state.project);
  const [fields, setFields] = useState<AdjustFields | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const valid =
    !!record &&
    !!project &&
    record.committed.projectId === project.id &&
    record.committed.revisionId === project.currentRevision;
  useEffect(() => {
    if (open && record) setFields(fieldsFromSession(record.base));
    setError(null);
  }, [open, record]);
  if (!open) return null;
  const close = () => setOpen(false);
  const apply = async () => {
    if (!record || !project || !fields) return;
    setError(null);
    const run = async () => {
      setBusy(true);
      try {
        const result = await amendLast(project, record, fields);
        setLast(result.record);
        onAdopt(result.project);
        return true;
      } catch (cause) {
        setError(errorText(cause));
        return false;
      } finally {
        setBusy(false);
      }
    };
    await (confirmEdit
      ? confirmEdit(
          t("Adjust last operation"),
          t(
            "Replace the parameters of the last transform. One Undo still returns to the state before it.",
          ),
          t(kindTitle[fields.kind]),
          run,
        )
      : run());
  };
  const panel = (
    <section className="adjust-last" aria-label={t("Adjust last operation")}>
      <header>
        <strong>
          {valid && fields
            ? t(kindTitle[fields.kind])
            : t("Adjust last operation")}
        </strong>
        <IconButton label={t("Close")} onClick={close}>
          <X size={14} />
        </IconButton>
      </header>
      {!valid || !fields ? (
        <p className="quiet">
          {t(
            "The last operation cannot be adjusted. Only the latest transform made in this session can be changed.",
          )}
        </p>
      ) : (
        <>
          {fields.kind === "move" &&
            axes.map((axis, index) => (
              <NumberField
                key={axis}
                label={`${axis.toUpperCase()}, ${t("mm")}`}
                value={fields.offsetMm[index]}
                onChange={(value) => {
                  const offsetMm = [...fields.offsetMm] as [
                    number,
                    number,
                    number,
                  ];
                  offsetMm[index] = value;
                  setFields({ ...fields, offsetMm });
                }}
              />
            ))}
          {fields.kind === "rotate" && (
            <NumberField
              label={t("Angle, °")}
              value={fields.angleDeg}
              onChange={(angleDeg) => setFields({ ...fields, angleDeg })}
            />
          )}
          {fields.kind === "scale" && (
            <NumberField
              label={t("Factor")}
              value={fields.factor}
              onChange={(factor) => setFields({ ...fields, factor })}
            />
          )}
          {fields.kind !== "move" && (
            <div
              className="adjust-last-axes"
              role="radiogroup"
              aria-label={t("Axis")}
            >
              {fields.kind === "scale" && (
                <button
                  role="radio"
                  aria-checked={fields.axis === null}
                  onClick={() => setFields({ ...fields, axis: null })}
                >
                  {t("All")}
                </button>
              )}
              {axes.map((axis) => (
                <button
                  key={axis}
                  role="radio"
                  aria-checked={fields.axis === axis}
                  onClick={() => setFields({ ...fields, axis })}
                >
                  {axis.toUpperCase()}
                </button>
              ))}
            </div>
          )}
          {error && (
            <p className="adjust-last-error" role="alert">
              {error}
            </p>
          )}
          <Button
            className="primary"
            disabled={busy}
            onClick={() => void apply()}
          >
            {busy ? t("Applying…") : t("Apply")}
          </Button>
        </>
      )}
    </section>
  );
  if (!valid && project?.currentRevision) {
    const program = project.revisions.find((revision) => revision.id === project.currentRevision)?.program;
    const feature = program && readTypedCadDocument(program)?.features.at(-1);
    if (feature && readDatumOperation(feature.operation)) {
      return <AdjustDatum key={`${project.id}:${project.currentRevision}`}
        projectId={project.id} revisionId={project.currentRevision}
        close={close} onAdopt={onAdopt} confirmEdit={confirmEdit} fallback={panel} />;
    }
    return <AdjustTorus key={`${project.id}:${project.currentRevision}`}
      projectId={project.id} revisionId={project.currentRevision}
      close={close} onAdopt={onAdopt} confirmEdit={confirmEdit} fallback={panel} />;
  }
  return panel;
}
