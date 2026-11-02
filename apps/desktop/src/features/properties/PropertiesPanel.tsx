import { useState, type ReactNode } from "react";
import { Button } from "../../components/ui";
import { t, number } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
export function PropertiesPanel({
  source,
  featureId,
  children,
  onEdit,
  initialTab = "body",
}: {
  source: string;
  featureId: string | null;
  children: ReactNode;
  onEdit: () => void;
  initialTab?: "body" | "parameters";
}) {
  const document = readTypedCadDocument(source),
    feature = document?.features.find((item) => item.id === featureId);
  const [tab, setTab] = useState<"body" | "parameters">(initialTab);
  return (
    <>
      <div
        className="panel-tabs"
        role="tablist"
        aria-label={t("Property context")}
      >
        <button
          role="tab"
          aria-selected={tab === "body"}
          onClick={() => setTab("body")}
        >
          {feature ? t("Operation") : t("Body")}
        </button>
        <button
          role="tab"
          aria-selected={tab === "parameters"}
          onClick={() => setTab("parameters")}
        >
          {t("Parameters")}
        </button>
      </div>
      {tab === "parameters" ? (
        <div className="workspace-content-list">
          <dl>
            {document?.parameters.map((parameter) => (
              <div key={parameter.id}>
                <dt>{parameter.name}</dt>
                <dd>
                  {number(parameter.valueMm)} {t("mm")}
                </dd>
              </div>
            ))}
          </dl>
          <Button onClick={onEdit}>{t("Edit model parameters")}</Button>
        </div>
      ) : feature ? (
        <div className="workspace-content-list">
          <h3>{feature.name}</h3>
          <code>{feature.id}</code>
          <p>{t(feature.operation.type)}</p>
          <Button onClick={onEdit}>{t("Edit operation")}</Button>
        </div>
      ) : (
        children
      )}
    </>
  );
}
