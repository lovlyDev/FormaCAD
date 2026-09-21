import { t } from "../i18n";
import { dependencies, readCadDocument } from "../lib/cadDocument";

/** A projection of the document; geometry remains exclusively in the backend. */
export function CadFeatureEditor({
  source,
  disabled,
  onChange,
}: {
  source: string;
  disabled: boolean;
  onChange: (source: string) => void;
}) {
  const doc = readCadDocument(source);
  if (!doc) return null;
  return (
    <div aria-label={t("CAD features")}>
      {doc.features.map((feature, index) => (
        <details key={feature.id}>
          <summary>
            {feature.id} · {t(feature.operation.type)}
            {doc.output === feature.id ? ` · ${t("Output")}` : ""}
          </summary>
          {dependencies(feature.operation).length > 0 && (
            <p className="field-hint">
              {t("References:")} {dependencies(feature.operation).join(", ")}
            </p>
          )}
          {Object.entries(feature.operation)
            .filter(([, value]) => typeof value === "number")
            .map(([name, value]) => (
              <label className="field" key={name}>
                {t(name)} ({t("mm")})
                <input
                  type="number"
                  step="any"
                  defaultValue={value as number}
                  key={`${feature.id}-${name}-${value}`}
                  disabled={disabled}
                  aria-label={`${feature.id}.${t(name)}`}
                  onBlur={(event) => {
                    if (
                      !event.target.value.trim() ||
                      !Number.isFinite(event.target.valueAsNumber)
                    )
                      return;
                    const features = doc.features.map((f, i) =>
                      i === index
                        ? {
                            ...f,
                            operation: {
                              ...f.operation,
                              [name]: event.target.valueAsNumber,
                            },
                          }
                        : f,
                    );
                    onChange(JSON.stringify({ ...doc, features }, null, 2));
                  }}
                />
              </label>
            ))}
        </details>
      ))}
    </div>
  );
}
