import { Checkbox } from "./ui";
import { Select } from "./ui";
import { Layers,Box,Link2 } from "lucide-react";
import { FeatureIcon } from "./FeatureIcon";
import { t,number } from "../i18n";
import { activeFeatureIds, canSuppressFeature, featureDependencies, readTypedCadDocument, solidFeature } from "../lib/typedCadDocument";
import { readSketchOperation } from "../lib/sketchDocument";
import { CadParameterEditor } from "./CadParameterEditor";
import { Sketch2dEditor } from "./Sketch2dEditor";
import { readTransformOperation } from "../features/feature-transforms/transformOperation";
import { TransformFeatureEditor } from "../features/feature-transforms/TransformFeatureEditor";
import "./TypedFeatureTree.css";

export function TypedFeatureTree({ source, disabled, onChange, failedFeatureId }: {
  source: string;
  disabled: boolean;
  onChange: (source: string) => void;
  failedFeatureId?: string | null;
}) {
  const document = readTypedCadDocument(source);
  if (!document) return null;
  const update = (next: typeof document) => onChange(JSON.stringify(next, null, 2));
  const solidOutputs = document.features.filter(solidFeature);
  const active = activeFeatureIds(document);
  return <section className="typed-feature-tree" aria-label={t("Feature history")}>
    <CadParameterEditor document={document} disabled={disabled} onChange={update} />
    <h3><Layers size={16}/>{t("Feature history")}<span>{number(document.features.length,0)}</span></h3>
    <ol>
      {document.features.map((feature) => {
        const dependencies = featureDependencies(feature);
        const sketch = readSketchOperation(feature.operation);
        const transform = readTransformOperation(feature.operation);
        return <li key={feature.id} className={[feature.suppressed?"is-suppressed":"",feature.id===failedFeatureId?"is-failed":""].filter(Boolean).join(" ")}>
          <div className="typed-feature-row">
            <span className="typed-feature-icon"><FeatureIcon type={feature.operation.type}/></span>
            <span className="typed-feature-name">{feature.name}</span>
            <span className="typed-feature-type">{t(feature.operation.type)}</span>
            <span className="typed-feature-status">{feature.suppressed ? t("Suppressed") : active.has(feature.id) ? t("Active feature") : t("After rollback")}</span>
            {canSuppressFeature(feature) && <Checkbox checked={!!feature.suppressed} disabled={disabled}
              onChange={checked=>update({...document,features:document.features.map(entry=>entry.id===feature.id?{...entry,suppressed:checked}:entry)})}>{t("Suppress")}</Checkbox>}

          </div>
          <div className="typed-feature-meta"><code>{feature.id}</code>{dependencies.length>0&&<span title={`${t("References:")} ${dependencies.join(", ")}`}><Link2 size={12}/>{dependencies.join(", ")}</span>}</div>
          {transform && <TransformFeatureEditor featureId={feature.id} operation={transform} disabled={disabled} onChange={operation => update({ ...document, features: document.features.map(entry => entry.id === feature.id ? { ...entry, operation } : entry) })} />}
          {sketch && <Sketch2dEditor operation={sketch} parameters={document.parameters} featureId={feature.id} usage={document.features.filter(entry=>featureDependencies(entry).includes(feature.id)).map(entry=>entry.name)} disabled={disabled}
            onChange={(operation) => update({ ...document, features: document.features.map((entry) =>
              entry.id === feature.id ? { ...entry, operation } : entry) })} />}
        </li>;
      })}
    </ol>
    {document.bodies.map((body) => <label key={body.id} className="typed-feature-output">
      <span><Box size={15}/>{body.name}<small>{t("Build up to")}</small></span>
      <Select value={body.sourceFeatureId} disabled={disabled}
        onChange={(event) => update({ ...document, bodies: document.bodies.map((entry) => entry.id === body.id ? { ...entry, sourceFeatureId: event.target.value } : entry) })}>
        {solidOutputs.map((feature) => <option key={feature.id} value={feature.id}>{feature.name}</option>)}
      </Select>
    </label>)}
  </section>;
}
