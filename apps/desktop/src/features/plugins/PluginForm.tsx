import { useId } from "react";
import { t } from "../../i18n";
import { Checkbox, NumberInput, Select } from "../../components/ui";
import type { FieldValue, FormField, Translations } from "./api";
import { fieldError, pluginText } from "./pluginText";

/** Declarative plugin form rendered with FormaCAD's own controls. */
export function PluginForm({
  fields,
  values,
  onChange,
  translations,
  disabled = false,
}: {
  fields: FormField[];
  values: Record<string, FieldValue>;
  onChange: (values: Record<string, FieldValue>) => void;
  translations: Translations;
  disabled?: boolean;
}) {
  const id = useId();
  if (!fields.length) return <p className="field-hint">{t("This action has no parameters.")}</p>;
  return (
    <div className="plugin-form">
      {fields.map((field) => {
        const label = pluginText(translations, field.label);
        const value = values[field.key];
        const error = fieldError(field, value);
        const set = (next: FieldValue) => onChange({ ...values, [field.key]: next });
        const help = field.help ? pluginText(translations, field.help) : null;
        const describedBy = `${id}-${field.key}-hint`;
        if (field.type === "boolean")
          return (
            <div className="plugin-field" key={field.key}>
              <Checkbox checked={value === true} disabled={disabled} onChange={set}>
                {label}
              </Checkbox>
              {help && <small id={describedBy}>{help}</small>}
            </div>
          );
        const unit = field.type === "length" ? t("mm") : field.type === "angle" ? "°" : "";
        return (
          <label className="plugin-field" key={field.key}>
            <span>
              {label}
              {unit && <em>{unit}</em>}
            </span>
            {field.type === "choice" ? (
              <Select aria-label={label} value={String(value)} disabled={disabled} onChange={(event) => set(event.target.value)}>
                {field.options?.map((option) => (
                  <option key={option.value} value={option.value}>
                    {pluginText(translations, option.label)}
                  </option>
                ))}
              </Select>
            ) : field.type === "text" ? (
              <input
                className="input"
                aria-label={label}
                aria-describedby={help ? describedBy : undefined}
                value={String(value ?? "")}
                maxLength={1000}
                disabled={disabled}
                onChange={(event) => set(event.target.value)}
              />
            ) : (
              <NumberInput
                aria-label={label}
                aria-invalid={!!error}
                aria-describedby={help || error ? describedBy : undefined}
                type="number"
                value={typeof value === "number" ? value : ""}
                min={field.min}
                max={field.max}
                step={field.type === "integer" ? 1 : field.type === "angle" ? 1 : 0.1}
                disabled={disabled}
                onChange={(event) => set(Number.isNaN(event.target.valueAsNumber) ? Number(event.target.value) : event.target.valueAsNumber)}
              />
            )}
            {(error || help) && (
              <small id={describedBy} className={error ? "error-inline" : undefined}>
                {error ?? help}
              </small>
            )}
          </label>
        );
      })}
    </div>
  );
}

export const formValid = (fields: FormField[], values: Record<string, FieldValue>) =>
  fields.every((field) => fieldError(field, values[field.key]) === null);
