import { LoaderCircle } from "lucide-react";
import { number, t } from "../../../i18n";
import { parseTransformNumber } from "../numericInput";
import { snapSessionIncrements } from "../../snapping/incrementSnap";
import { transformIntents, type TransformSession } from "../transformSession";
import { useTransformViewport } from "./transformStore";
import "./transformHud.css";

const kindLabel: Record<TransformSession["kind"], string> = {
  move: "Move",
  rotate: "Rotate",
  scale: "Scale",
};

function axisLabel(session: TransformSession): string {
  const constraint = session.constraint;
  if (!constraint) return t("Free");
  const axis = constraint.axis.toUpperCase();
  const space = constraint.space === "local" ? t("local") : t("global");
  return constraint.exclude
    ? t("Plane without {{value0}} ({{value1}})", {
        value0: axis,
        value1: space,
      })
    : t("Axis {{value0}} ({{value1}})", { value0: axis, value1: space });
}

export function transformValue(
  session: TransformSession,
  snap: boolean,
): string {
  const intents = transformIntents(snapSessionIncrements(session, snap));
  const intent = intents?.[0]?.intent;
  if (!intent) return t("Invalid value");
  if (intent.kind === "move")
    return intent.offsetMm
      .map((value) => `${number(value, 3)} ${t("mm")}`)
      .join(" · ");
  if (intent.kind === "rotate") return `${number(intent.angleDeg, 2)}°`;
  return intent.factors.map((value) => `×${number(value, 3)}`).join(" · ");
}

/** Status line for the active modal tool: operation, constraint, typed value and hints. */
export function TransformHud() {
  const session = useTransformViewport((state) => state.session);
  const status = useTransformViewport((state) => state.status);
  const snap = useTransformViewport((state) => state.incrementSnap);
  const snapTarget = useTransformViewport((state) => state.snapTarget);
  if (!session) {
    if (status.kind === "committing")
      return (
        <div className="transform-hud" role="status">
          <LoaderCircle size={14} className="spin" />
          {t("Applying transform…")}
        </div>
      );
    if (status.kind === "error")
      return (
        <div className="transform-hud error" role="alert">
          {status.message}
        </div>
      );
    return null;
  }
  const quantity =
    session.kind === "move"
      ? "length"
      : session.kind === "rotate"
        ? "angle"
        : "scale";
  const input = parseTransformNumber(session.input, quantity);
  return (
    <div className="transform-hud" role="status" aria-live="polite">
      <strong>{t(kindLabel[session.kind])}</strong>
      <span>{axisLabel(session)}</span>
      {snapTarget === "vertex" && !session.input && (
        <span className="snapped">{t("Snapped to vertex")}</span>
      )}
      <span className={input.status === "invalid" ? "invalid" : ""}>
        {session.input
          ? `${t("Value")}: ${session.input}`
          : transformValue(session, snap)}
      </span>
      <small>
        {t(
          "X/Y/Z axis · Shift+axis plane · type a value · Shift precise · Ctrl increments · Enter/LMB confirm · Esc/RMB cancel",
        )}
      </small>
    </div>
  );
}
