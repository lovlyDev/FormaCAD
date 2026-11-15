import { t } from "../../i18n";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";
import { commitIges, type IgesInput } from "./igesApi";
export function captureIgesImport(project: Project, input: IgesInput, sourceSha256: string) {
  if (input.projectId !== project.id || input.expectedRevision !== project.currentRevision || !/^[a-f0-9]{64}$/.test(sourceSha256)) return null;
  const prompt = t("Imported IGES: {{value0}}", { value0: input.name });
  const captured = captureApply(project, "", prompt);
  const request=Object.freeze({...input});
  return { captured, invoke: () => commitIges(request, sourceSha256, prompt) };
}
