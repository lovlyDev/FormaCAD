import { invoke } from "@tauri-apps/api/core";
import { Mesh, type Object3D } from "three";
import { number, t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";

export function captureSeparateSolids(project: Project, mesh: Object3D) {
  if (!(mesh instanceof Mesh) || !mesh.visible || mesh.userData.formaSelectionLocked) return null;
  const count: unknown = mesh.geometry.userData.formaSolidCount;
  if (typeof count !== "number" || !Number.isInteger(count) || count < 2 || count > 128) return null;
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  const body = document?.bodies.find((item) => item.id === mesh.name);
  if (!body || document?.revisionId !== project.currentRevision) return null;
  const captured = captureApply(project, "", t("Separate solids"));
  return {
    captured, name: body.name,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt,
      commands: [{ command: "separate_solids", bodyId: body.id,
        newBodies: Array.from({ length: count - 1 }, (_, index) => ({
          id: crypto.randomUUID(),
          name: t("{{name}} · part {{index}}", {
            name: Array.from(body.name).slice(0, 90).join(""), index: number(index + 2, 0),
          }),
        })),
      }],
    }),
  };
}
