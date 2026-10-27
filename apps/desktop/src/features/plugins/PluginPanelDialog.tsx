import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { LoaderCircle } from "lucide-react";
import { errorText, t } from "../../i18n";
import { Button, Modal } from "../../components/ui";
import { renderPanel, type FieldValue, type PanelBlock } from "./api";
import { formValid, PluginForm } from "./PluginForm";
import { usePluginUi } from "./pluginUi";
import { initialValues, pluginText } from "./pluginText";

/** Declarative plugin panel. Text is shown as plain text; forms submit plugin commands. */
export function PluginPanelDialog({
  projectId,
  disabled,
  onCommand,
}: {
  projectId: string | null;
  disabled: boolean;
  onCommand: (pluginId: string, commandId: string, values: Record<string, FieldValue>) => void;
}) {
  const panel = usePluginUi((state) => state.panel);
  const close = usePluginUi((state) => state.openPanel);
  const view = useQuery({
    queryKey: ["plugin-panel", panel?.plugin.manifest.id, panel?.panelId, projectId],
    queryFn: () => renderPanel(projectId, panel!.plugin.manifest.id, panel!.panelId),
    enabled: !!panel,
    staleTime: 0,
  });
  if (!panel) return null;
  const translations = panel.plugin.translations;
  const definition = panel.plugin.manifest.contributes.panels.find((item) => item.id === panel.panelId);
  return (
    <Modal
      open
      onClose={() => close(null)}
      title={pluginText(translations, definition?.title ?? panel.panelId)}
      description={t("Plugin {{value0}} {{value1}}", { value0: pluginText(translations, panel.plugin.manifest.name), value1: panel.plugin.active })}
    >
      {view.isFetching && <LoaderCircle size={16} className="spin" />}
      {view.error && <p className="error-inline">{errorText(view.error)}</p>}
      {view.data?.view.blocks.map((block, index) => (
        <PanelBlockView
          key={index}
          block={block}
          translations={translations}
          disabled={disabled}
          onSubmit={(values) => {
            if (block.type !== "form") return;
            close(null);
            onCommand(panel.plugin.manifest.id, block.submit.command, values);
          }}
        />
      ))}
    </Modal>
  );
}

function PanelBlockView({
  block,
  translations,
  disabled,
  onSubmit,
}: {
  block: PanelBlock;
  translations: Record<string, Record<string, string>>;
  disabled: boolean;
  onSubmit: (values: Record<string, FieldValue>) => void;
}) {
  const [values, setValues] = useState(() => (block.type === "form" ? initialValues(block.fields) : {}));
  if (block.type === "text") return <p className="plugin-panel-text">{pluginText(translations, block.text)}</p>;
  return (
    <div className="plugin-panel-form">
      <PluginForm fields={block.fields} values={values} onChange={setValues} translations={translations} disabled={disabled} />
      <div className="modal-actions">
        <Button className="primary" disabled={disabled || !formValid(block.fields, values)} onClick={() => onSubmit(values)}>
          {pluginText(translations, block.submit.label)}
        </Button>
      </div>
    </div>
  );
}
