import { useQuery } from "@tanstack/react-query";
import { AlertCircle, Box, SlidersHorizontal } from "lucide-react";
import { t, fixedNumber, getLocale } from "../i18n";
import { native } from "../lib/api";
import { inspectSavedModel } from "../lib/modelInspection";
import type { inspectModel } from "../lib/model";
import type { Project, Revision } from "../types";
import type { FaceSelection } from "../features/viewer/faceSelection";
import type { EdgeSelection } from "../features/viewer/edgeSelection";
import "./ModelInspector.css";

interface Props {
  project: Project;
  revision: Revision | undefined;
  selectedLabel: string | null;
  selectedFace: FaceSelection | null;
  selectedFaceAreaMm2: number | null;
  selectedEdge: EdgeSelection | null;
  selectedEdgeLengthMm: number | null;
  selectedEdgeRadiusMm: number | null;
  stats: ReturnType<typeof inspectModel>;
  thickness: number;
  onEditParameters: () => void;
}

export function ModelInspector({
  project,
  revision,
  selectedLabel,
  selectedFace,
  selectedFaceAreaMm2,
  selectedEdge,
  selectedEdgeLengthMm,
  selectedEdgeRadiusMm,
  stats,
  thickness,
  onEditParameters,
}: Props) {
  const source = revision?.source ?? "";
  const exact = useQuery({
    queryKey: ["exact-model-properties", project.id, revision?.id],
    queryFn: () => inspectSavedModel(project.id, revision!.id),
    enabled: native && !!revision && /\.st(e)?p$/i.test(source),
    retry: false,
    staleTime: Infinity,
  });
  const properties = exact.data;
  const dimensions = properties?.boundsMm ?? stats.size;
  const unitFactor =
    project.units === "inch" ? 25.4 : project.units === "cm" ? 10 : 1;
  const areaUnit =
    project.units === "inch" ? "in²" : project.units === "cm" ? "cm²" : "mm²";

  return (
    <div className="inspector">
      <div className="inspector-title">
        <Box size={13} />
        <span>{t("Model properties")}</span>
        <button aria-label={t("Edit parameters")} onClick={onEditParameters}>
          <SlidersHorizontal size={13} />
        </button>
      </div>
      {selectedLabel && (
        <small>{t("Selected: {{value0}}", { value0: selectedLabel })}</small>
      )}
      {selectedFace && selectedFace.revisionId === revision?.id && (
        <small>
          {t(
            "Face selection belongs to this revision and is cleared after a model change.",
          )}
        </small>
      )}
      {selectedEdge && selectedEdge.revisionId === revision?.id && (
        <small>
          {t(
            "Edge selection belongs to this revision and is cleared after a model change.",
          )}
        </small>
      )}
      <dl>
        <div>
          <dt>
            {properties ? t("CAD dimensions (X × Y × Z)") : t("Dimensions")}
          </dt>
          <dd>
            {dimensions.map((n) => fixedNumber(n / unitFactor, 2)).join(" × ")}{" "}
            <span>{t(project.units)}</span>
          </dd>
        </div>
        <div>
          <dt>{t("Bodies")}</dt>
          <dd>{stats.bodies}</dd>
        </div>
        <div>
          <dt>{t("Triangles")}</dt>
          <dd>{stats.triangles.toLocaleString(getLocale())}</dd>
        </div>
        {selectedFace &&
          selectedFace.revisionId === revision?.id &&
          selectedFaceAreaMm2 !== null && (
            <div>
              <dt>{t("Exact CAD face area")}</dt>
              <dd>
                {fixedNumber(
                  selectedFaceAreaMm2 / (unitFactor * unitFactor),
                  3,
                )}{" "}
                {t(areaUnit)}
              </dd>
            </div>
          )}
        {selectedEdge &&
          selectedEdge.revisionId === revision?.id &&
          selectedEdgeLengthMm !== null && (
            <div>
              <dt>{t("Exact CAD edge length")}</dt>
              <dd>
                {fixedNumber(selectedEdgeLengthMm / unitFactor, 3)}{" "}
                {t(project.units)}
              </dd>
            </div>
          )}
        {selectedEdge &&
          selectedEdge.revisionId === revision?.id &&
          selectedEdgeRadiusMm !== null && (
            <>
              <div>
                <dt>{t("Exact CAD edge radius")}</dt>
                <dd>
                  {fixedNumber(selectedEdgeRadiusMm / unitFactor, 3)}{" "}
                  {t(project.units)}
                </dd>
              </div>
              <div>
                <dt>{t("Exact CAD edge diameter")}</dt>
                <dd>
                  {fixedNumber((selectedEdgeRadiusMm * 2) / unitFactor, 3)}{" "}
                  {t(project.units)}
                </dd>
              </div>
            </>
          )}
        {properties ? (
          <>
            <div>
              <dt>{t("CAD volume")}</dt>
              <dd>
                {fixedNumber(properties.volumeMm3 / 1000, 2)} {t("cm³")}
              </dd>
            </div>
            <div>
              <dt>{t("CAD surface area")}</dt>
              <dd>
                {fixedNumber(properties.areaMm2 / 100, 2)} {t("cm²")}
              </dd>
            </div>
            <div>
              <dt>{t("Faces")}</dt>
              <dd>{properties.faceCount.toLocaleString(getLocale())}</dd>
            </div>
            <div>
              <dt>{t("Edges")}</dt>
              <dd>{properties.edgeCount.toLocaleString(getLocale())}</dd>
            </div>
          </>
        ) : (
          <div>
            <dt>{t("Surface area¹")}</dt>
            <dd>
              {fixedNumber(stats.area / 100, 1)} {t("cm²")}
            </dd>
          </div>
        )}
      </dl>
      {properties ? (
        <small>{t("Exact CAD properties describe the whole model.")}</small>
      ) : exact.isPending && exact.fetchStatus === "fetching" ? (
        <small>{t("Calculating CAD properties…")}</small>
      ) : exact.isError ? (
        <small>
          {t("Exact CAD properties unavailable; mesh estimate shown.")}
        </small>
      ) : (
        <small>{t("¹ Mesh estimate; overlapping faces included.")}</small>
      )}
      {thickness < 1 && (
        <div className="thin-warning">
          <AlertCircle size={13} />
          {t("Thin walls may be difficult to print.")}
        </div>
      )}
    </div>
  );
}
