import "./plugins.css";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type * as THREE from "three";
import { errorText, t } from "../../i18n";
import { native } from "../../lib/api";
import { useWorkspace } from "../../stores/workspace";
import type { Project } from "../../types";
import {
  exportWithPlugin,
  importWithPlugin,
  installFromLink,
  listPlugins,
  runPlugin,
  takePendingLink,
  type FieldValue,
  type Notice,
} from "./api";
import { OrbitCaptureDialog, type CaptureRequest } from "./capture/OrbitCaptureDialog";
import { PluginInstallDialog } from "./PluginInstallDialog";
import { PluginPanelDialog } from "./PluginPanelDialog";
import { contributionOf, PluginRunDialog } from "./PluginRunDialog";
import { usePluginUi, type RunTarget } from "./pluginUi";
import { pluginText } from "./pluginText";

const safeFileName = (name: string) => name.replace(/[^\p{L}\p{N}_-]/gu, "-");

type Ask = (title: string, description: string, detail: string, run: () => Promise<void>, action?: string) => Promise<void>;

/** Current viewer selection in the shape documented for `selection.current`. */
export function selectionForPlugins(): unknown[] {
  const { selected, selectedFace, selectedEdge } = useWorkspace.getState();
  if (selectedFace) return [{ kind: "face", bodyId: selectedFace.bodyId, key: `face:${selectedFace.faceOrdinal}` }];
  if (selectedEdge)
    return [{ kind: "edge", bodyId: selectedEdge.bodyId, key: selectedEdge.semanticKey ?? `edge:${selectedEdge.edgeOrdinal}` }];
  return selected ? [{ kind: "body", bodyId: selected, key: selected }] : [];
}

/**
 * Plugin dialogs and `formacad://` links. Model changes use the app's usual
 * approval (`ask`) and the host applies them as one revision.
 */
export function PluginHost({
  project,
  disabled,
  ask,
  notify,
  fail,
  scene,
}: {
  project: Project | null;
  disabled: boolean;
  ask: Ask;
  notify: (message: string) => void;
  fail: (message: string) => void;
  /** Model shown in the 3D view, for `viewport.capture` recordings. */
  scene: () => THREE.Object3D | null;
}) {
  const showPreview = usePluginUi((state) => state.showPreview);
  const [capture, setCapture] = useState<CaptureRequest | null>(null);

  useEffect(() => {
    if (!native) return;
    let active = true;
    const openPending = async () => {
      const url = await takePendingLink();
      if (!url) return;
      try {
        const preview = await installFromLink(url);
        if (active) showPreview(preview);
      } catch (error) {
        if (active) fail(errorText(error));
      }
    };
    void openPending();
    const stop = listen<string>("plugins://install-link", () => void openPending());
    return () => {
      active = false;
      void stop.then((unlisten) => unlisten());
    };
  }, [showPreview, fail]);

  const report = (pluginName: string, notices: Notice[], message?: string | null) => {
    for (const notice of [...(message ? [{ level: "info", message }] : []), ...notices])
      notify(t("{{value0}}: {{value1}}", { value0: pluginName, value1: notice.message }));
  };

  const run = (target: RunTarget, values: Record<string, FieldValue>) => {
    const base = useWorkspace.getState().project;
    const contribution = contributionOf(target);
    if (!base || !contribution) {
      fail(t("Open a project to use plugin actions."));
      return;
    }
    const name = pluginText(target.plugin.translations, target.plugin.manifest.name);
    const title = pluginText(target.plugin.translations, contribution.title);
    const prompt = t("Plugin {{value0}} {{value1}}: {{value2}}", { value0: name, value1: target.plugin.active, value2: title });
    const setProject = useWorkspace.getState().setProject;
    if (target.kind === "export") {
      void ask(t("Export with a plugin"), t("The plugin receives the current model and returns the file contents."), prompt, async () => {
        const result = await exportWithPlugin({
          project: base,
          pluginId: target.plugin.manifest.id,
          exporterId: target.contributionId,
          params: values,
          fileName: safeFileName(base.name),
        });
        report(name, result.notices);
        notify(result.path ? t("Exported {{value0}}", { value0: result.path }) : t("Export cancelled"));
      }, "export_file");
      return;
    }
    const execute = async () => {
        const outcome =
          target.kind === "import"
            ? await importWithPlugin({
                project: base,
                pluginId: target.plugin.manifest.id,
                importerId: target.contributionId,
                params: values,
                prompt,
                title: t("Choose a file to import"),
              })
            : await runPlugin({
                project: base,
                pluginId: target.plugin.manifest.id,
                kind: target.kind === "command" ? "command" : "feature",
                contributionId: target.contributionId,
                params: values,
                prompt,
                selection: selectionForPlugins(),
              });
        if (!outcome) return;
        if (outcome.project) setProject(outcome.project);
        report(name, outcome.notices, outcome.message);
        if (outcome.capture)
          setCapture({ pluginName: name, fileName: safeFileName(base.name), ticket: outcome.capture });
    };
    // A command without `model.write` cannot change the model, so it needs no edit approval.
    if (target.kind === "command" && !target.plugin.acceptedPermissions.includes("model.write")) {
      void execute().catch((error: unknown) => fail(errorText(error)));
      return;
    }
    void ask(
      t("Run plugin operation"),
      t("The plugin proposes CAD operations. FormaCAD checks them, builds the geometry and saves one revision."),
      prompt,
      execute,
    );
  };

  return (
    <>
      <PluginInstallDialog onInstalled={notify} />
      <PluginRunDialog disabled={disabled} onRun={run} />
      {capture && (
        <OrbitCaptureDialog request={capture} scene={scene} onDone={() => setCapture(null)} notify={notify} fail={fail} />
      )}
      <PluginPanelDialog
        projectId={project?.id ?? null}
        disabled={disabled}
        onCommand={(pluginId, commandId, values) => {
          void listPlugins().then((plugins) => {
            const plugin = plugins.find((item) => item.manifest.id === pluginId);
            if (plugin) run({ plugin, kind: "command", contributionId: commandId }, values);
          });
        }}
      />
    </>
  );
}
