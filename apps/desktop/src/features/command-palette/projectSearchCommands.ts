import { t, number } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
export function projectSearchCommands(
  project: Project | null,
  navigate: (
    kind: "body" | "feature" | "parameter" | "revision",
    id: string,
  ) => void,
) {
  if (!project) return [];
  const source =
    project.revisions.find((item) => item.id === project.currentRevision)
      ?.program ?? "";
  const document = readTypedCadDocument(source);
  return [
    ...(document?.bodies ?? []).map((body) => ({
      label: `${t("Body")}: ${body.name} · ${body.id}`,
      run: () => navigate("body", body.id),
    })),
    ...(document?.features ?? []).map((feature) => ({
      label: `${t("Operation")}: ${feature.name} · ${feature.id} · ${t(feature.operation.type)}`,
      run: () => navigate("feature", feature.id),
    })),
    ...(document?.parameters ?? []).map((parameter) => ({
      label: `${t("Parameters")}: ${parameter.name} · ${parameter.id} · ${number(parameter.valueMm)} ${t("mm")}`,
      run: () => navigate("parameter", parameter.id),
    })),
    ...project.revisions.map((revision, index) => ({
      label: `${t("Revision")} ${number(index + 1, 0)} · ${revision.prompt}`,
      run: () => navigate("revision", revision.id),
    })),
  ];
}
