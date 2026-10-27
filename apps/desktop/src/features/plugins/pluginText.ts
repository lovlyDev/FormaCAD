import { getLocale, t } from "../../i18n";
import type { FieldValue, FormField, Translations } from "./api";

/**
 * Manifest strings are author content and are never passed to t().
 * `%key%` strings are resolved from the plugin's own locales/<lang>.json.
 */
export function pluginText(translations: Translations, text: string): string {
  const key = /^%([A-Za-z0-9_.-]+)%$/.exec(text)?.[1];
  if (!key) return text;
  const locale = getLocale();
  return translations[locale]?.[key] ?? translations.en?.[key] ?? translations.ru?.[key] ?? key;
}

/** Plain-language description of a permission for consent dialogs. */
export function permissionText(permission: string): string {
  if (permission.startsWith("network:"))
    return t("Send HTTPS requests to {{value0}}", { value0: permission.slice("network:".length) });
  switch (permission) {
    case "model.read":
      return t("Read the current model: parameters, operations, bodies");
    case "model.write":
      return t("Propose model changes; they are applied as one undoable revision");
    case "selection.read":
      return t("See which bodies, faces or edges you selected");
    case "file.import":
      return t("Read the file you choose in the import dialog");
    case "file.export":
      return t("Write data to the file you choose in the save dialog");
    case "storage":
      return t("Keep its own data in the FormaCAD profile (up to 10 MB)");
    case "notifications":
      return t("Show notifications in FormaCAD");
    case "viewport.capture":
      return t("Record turntable videos of the model shown in the 3D view");
    default:
      return permission;
  }
}

/** Initial values: saved values first, then defaults, then neutral values. */
export function initialValues(fields: FormField[], saved: Record<string, FieldValue> = {}): Record<string, FieldValue> {
  return Object.fromEntries(
    fields.map((field) => {
      const value =
        saved[field.key] ??
        field.default ??
        (field.type === "boolean"
          ? false
          : field.type === "text"
            ? ""
            : field.type === "choice"
              ? (field.options?.[0]?.value ?? "")
              : Math.max(0, field.min ?? 0));
      return [field.key, value];
    }),
  );
}

/** Same checks as the host, so the user sees the problem before the call. */
export function fieldError(field: FormField, value: FieldValue | undefined): string | null {
  if (field.type === "boolean") return typeof value === "boolean" ? null : t("Enter a valid value");
  if (field.type === "text") return typeof value === "string" && value.length <= 1000 ? null : t("Enter a valid value");
  if (field.type === "choice")
    return field.options?.some((option) => option.value === value) ? null : t("Choose one of the options");
  if (typeof value !== "number" || !Number.isFinite(value)) return t("Enter a number");
  if (field.type === "integer" && !Number.isInteger(value)) return t("Enter a whole number");
  if ((field.min !== undefined && value < field.min) || (field.max !== undefined && value > field.max))
    return t("Allowed range: {{value0}} – {{value1}}", { value0: field.min ?? "−∞", value1: field.max ?? "∞" });
  return null;
}
