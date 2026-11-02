import { t } from "../../i18n";
export function revisionHistoryLabel(prompt: string): string | null {
  const match = /^history:(undo|redo|amend):([A-Za-z0-9_-]{1,80})$/.exec(
    prompt,
  );
  if (!match) return null;
  if (match[1] === "amend")
    return t("Adjusted operation from revision {{value0}}", {
      value0: match[2],
    });
  if (match[2] === "empty") return t("Undo to empty model");
  return t(
    match[1] === "undo"
      ? "Undo to revision {{value0}}"
      : "Redo to revision {{value0}}",
    { value0: match[2] },
  );
}
