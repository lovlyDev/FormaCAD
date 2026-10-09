import { useQuery } from "@tanstack/react-query";
import { t, fixedNumber } from "../i18n";
import { native } from "../lib/api";
import { compareCadRevisions } from "../lib/cadDiff";
import { inspectSavedModel, type ExactModelProperties } from "../lib/modelInspection";
import type { Project } from "../types";
import type { CadDocumentDiff } from "../lib/cadDiff";
import "./RevisionComparison.css";

function delta(before: number, after: number, divisor: number, digits: number): string {
  const value = (after - before) / divisor;
  return `${value > 0 ? "+" : ""}${fixedNumber(value, digits)}`;
}

function metricRow(label: string, before: number, after: number, divisor: number, unit: string, digits: number) {
  return <div className="revision-diff-metric" key={label}>
    <dt>{label}</dt>
    <dd>{fixedNumber(before / divisor, digits)} → {fixedNumber(after / divisor, digits)} {unit}
      <strong>{delta(before, after, divisor, digits)}</strong></dd>
  </div>;
}

function GeometryChanges({ before, after }: { before: ExactModelProperties; after: ExactModelProperties }) {
  return <dl className="revision-diff-metrics">
    {metricRow(t("CAD volume"), before.volumeMm3, after.volumeMm3, 1000, t("cm³"), 2)}
    {metricRow(t("CAD surface area"), before.areaMm2, after.areaMm2, 100, t("cm²"), 2)}
    {metricRow(t("Faces"), before.faceCount, after.faceCount, 1, "", 0)}
    {metricRow(t("Edges"), before.edgeCount, after.edgeCount, 1, "", 0)}
  </dl>;
}

function featureChange(change: CadDocumentDiff["features"][number]): string {
  if (!change.before || !change.after) return change.after ? t("Added") : t("Removed");
  if (!!change.before.value.suppressed !== !!change.after.value.suppressed)
    return change.after.value.suppressed ? t("Suppressed") : t("Resumed");
  if (JSON.stringify(change.before.value.operation) !== JSON.stringify(change.after.value.operation))
    return t("Operation changed");
  if (change.before.index !== change.after.index) return t("Moved in history");
  return t("Renamed");
}

export function RevisionComparison({ project, fromRevisionId, toRevisionId }: {
  project: Project;
  fromRevisionId: string;
  toRevisionId: string;
}) {
  const earlier = project.revisions.find((revision) => revision.id === fromRevisionId);
  const later = project.revisions.find((revision) => revision.id === toRevisionId);
  const structural = useQuery({
    queryKey: ["cad-ir-diff", project.id, fromRevisionId, toRevisionId],
    queryFn: () => compareCadRevisions(project.id, fromRevisionId, toRevisionId),
    enabled: native && !!earlier && !!later,
    staleTime: Infinity,
    retry: false,
  });
  const before = useQuery({
    queryKey: ["exact-model-properties", project.id, fromRevisionId],
    queryFn: () => inspectSavedModel(project.id, fromRevisionId),
    enabled: native && !!earlier?.source && /\.st(e)?p$/i.test(earlier.source),
    staleTime: Infinity,
    retry: false,
  });
  const after = useQuery({
    queryKey: ["exact-model-properties", project.id, toRevisionId],
    queryFn: () => inspectSavedModel(project.id, toRevisionId),
    enabled: native && !!later?.source && /\.st(e)?p$/i.test(later.source),
    staleTime: Infinity,
    retry: false,
  });
  const diff = structural.data;
  return <section className="revision-comparison" aria-label={t("CAD change summary")}>
    <h3>{t("CAD change summary")}</h3>
    {before.data && after.data ? <GeometryChanges before={before.data} after={after.data} /> :
      before.isFetching || after.isFetching ? <p>{t("Calculating CAD properties…")}</p> :
      <p>{t("Exact geometry comparison unavailable")}</p>}
    {diff ? <div className="revision-diff-structure">
      {diff.parameters.map((change) => <p key={`parameter-${change.id}`}>
        <strong>{change.after?.value.name ?? change.before?.value.name ?? change.id}</strong>
        {change.before && change.after ? <>: {fixedNumber(change.before.value.valueMm, 2)} → {fixedNumber(change.after.value.valueMm, 2)} {t("mm")}</> :
          <> · {change.after ? t("Added") : t("Removed")}</>}
      </p>)}
      {diff.features.map((change) => <p key={`feature-${change.id}`}>
        <strong>{change.after?.value.name ?? change.before?.value.name ?? change.id}</strong>
        <> · {featureChange(change)}</>
      </p>)}
      {diff.bodies.map((change) => <p key={`body-${change.id}`}>
        <strong>{change.after?.value.name ?? change.before?.value.name ?? change.id}</strong>
        {change.before && change.after ? change.before.value.sourceFeatureId !== change.after.value.sourceFeatureId ?
          <> · {t("Output")}: {change.before.value.sourceFeatureId} → {change.after.value.sourceFeatureId}</> :
          <> · {t("Renamed")}</> :
          <> · {change.after ? t("Added") : t("Removed")}</>}
      </p>)}
      {!diff.parameters.length && !diff.features.length && !diff.bodies.length && <p>{t("No structural CAD changes")}</p>}
    </div> : structural.isFetching ? <p>{t("Comparing saved revisions…")}</p> :
      <p>{t("Structure unavailable for legacy revisions")}</p>}
  </section>;
}
