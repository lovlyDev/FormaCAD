import { useMemo, useState } from "react";
import { Layers, Code2, SlidersHorizontal, Box } from "lucide-react";
import { t, fixedNumber } from "../../../i18n";
import { EditorDisclosure } from "../../model-editor/EditorDisclosure";
import { FeatureIcon } from "../../../components/FeatureIcon";
import "../../../components/TypedFeatureTree.css";
import "../../../components/CadParameterEditor.css";
import type { Project } from "../../../types";
import { candidateDiff, cadValuesEqual } from "./candidateDiff";

export function CandidateChanges({ base, source }: { base: Project; source: string }) {
  const diff = useMemo(() => candidateDiff(base, source), [base, source]);
  const [details, setDetails] = useState(false), [code, setCode] = useState(false), [parameters, setParameters] = useState(true);
  const total = diff ? diff.parameters.length + diff.features.length + diff.bodies.length : 0;
  return <section className="typed-feature-tree candidate-changes" aria-label={t("CAD change summary")}>
    <h3><Layers size={16} />{t("CAD change summary")}</h3>
    <p className="candidate-muted">{t("Changes are compared by stable CAD IDs. Geometry is shown separately.")}</p>
    {diff ? <>
      <EditorDisclosure className="cad-parameter-editor" icon={SlidersHorizontal} title={t("Named CAD parameters")} hint={fixedNumber(diff.parameters.length, 0)} hintClassName="cad-section-count" open={parameters} onOpenChange={setParameters}>
      <div className="candidate-parameter-changes">{diff.parameters.map(change => <div className="candidate-change" key={`parameter-${change.id}`}>
        <strong>{change.after?.value.name ?? change.before?.value.name}</strong><code>{change.id}</code>
        <span>{change.before && change.after
          ? `${fixedNumber(change.before.value.valueMm, 2)} → ${fixedNumber(change.after.value.valueMm, 2)} ${t("mm")}`
          : `${change.after ? t("Added") : t("Removed")} · ${fixedNumber((change.after ?? change.before)!.value.valueMm, 2)} ${t("mm")}`}</span>
      </div>)}{!diff.parameters.length && <p className="candidate-muted">{t("No parameter changes")}</p>}</div>
      </EditorDisclosure>
      <h3 className="candidate-section-title"><Layers size={16} />{t("Feature history")} <span>{fixedNumber(diff.features.length, 0)}</span></h3>
      <ol>{diff.features.map(change => <li className="candidate-change" key={`feature-${change.id}`}>
        <div className="typed-feature-row"><span className="typed-feature-icon"><FeatureIcon type={(change.after ?? change.before)!.value.operation.type} /></span>
        <strong className="typed-feature-name">{change.after?.value.name ?? change.before?.value.name}</strong>
        <span className="typed-feature-status">{!change.before || !change.after ? change.after ? t("Added") : t("Removed") : t("Modified")}</span></div>
        <div className="typed-feature-meta"><code>{change.id}</code></div>
        <div className="candidate-feature-changes">{!change.before || !change.after ? change.after ? t("Added") : t("Removed")
          : <>{!cadValuesEqual(change.before.value.operation, change.after.value.operation) && <span>{t("Operation changed")} · </span>}
            {!!change.before.value.suppressed !== !!change.after.value.suppressed && <span>{change.after.value.suppressed ? t("Suppressed") : t("Resumed")} · </span>}
            {change.before.index !== change.after.index && <span>{t("Moved in history")} · </span>}
            {change.before.value.name !== change.after.value.name && <span>{t("Renamed")} · </span>}
            <span>{t("Modified")}</span></>}
      </div></li>)}</ol>
      <h3 className="candidate-section-title"><Box size={16} />{t("Bodies")} <span>{fixedNumber(diff.bodies.length, 0)}</span></h3>
      {diff.bodies.map(change => <div className="candidate-change candidate-body-change" key={`body-${change.id}`}>
        <div className="typed-feature-row"><span className="typed-feature-icon"><Box size={17} /></span><strong className="typed-feature-name">{change.after?.value.name ?? change.before?.value.name}</strong></div><code>{change.id}</code>
        <span>{!change.before || !change.after ? change.after ? t("Added") : t("Removed") : t("Modified")}</span>
      </div>)}
      {!total && <p>{t("No structural CAD changes")}</p>}
      {!!total && <EditorDisclosure icon={Layers} title={t("Detailed CAD changes")} hint={t("Advanced")} open={details} onOpenChange={setDetails}>
        <pre>{JSON.stringify(diff, null, 2)}</pre></EditorDisclosure>}
    </> : <p>{t("Structure unavailable for legacy revisions")}</p>}
    <EditorDisclosure icon={Code2} title={t("Candidate CAD source")} hint={t("Advanced")} open={code} onOpenChange={setCode}>
      <div className="model-editor-code-toolbar"><span>{t("JSON · CAD IR v2")}</span><span>{t("Read-only")}</span></div>
      <pre>{source}</pre></EditorDisclosure>
  </section>;
}
