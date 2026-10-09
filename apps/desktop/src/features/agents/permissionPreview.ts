import { t } from "../../i18n";
import { getCustomAgentConfig } from "../../lib/api";
import type { Project } from "../../types";

export async function agentPermissionPreview(project: Project, request: string) {
  if (project.agent === "custom") {
    const config = await getCustomAgentConfig();
    if (!config) throw new Error(t("Custom CLI is not configured"));
    return {
      description: t("The configured executable runs with your account permissions and receives this request, recent chat, and the current model on stdin."),
      detail: t("Executable: {{value0}}\nArguments: {{value1}}\nProject: {{value2}}\n\nRequest: {{value3}}", {
        value0: config.executable,
        value1: config.args.join(" | ") || t("None"),
        value2: project.name,
        value3: request,
      }),
    };
  }
  return {
    description: t(
      "The local CLI sends your request and current model source using your existing login.",
    ),
    detail: t(
      "{{value0}} · {{value1}}\n\n{{value2}}\n\nThe agent writes CAD source for this request.",
      { value0: project.agent, value1: project.name, value2: request },
    ),
  };
}
