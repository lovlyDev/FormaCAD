import { useEffect, useState } from "react";
import { t } from "../../i18n";
import { Button, Modal } from "../../components/ui";
import type { FieldValue, FormField } from "./api";
import { formValid, PluginForm } from "./PluginForm";
import { usePluginUi, type RunTarget } from "./pluginUi";
import { initialValues, pluginText } from "./pluginText";

export function contributionOf(target: RunTarget): { title: string; params: FormField[] } | null {
  const c = target.plugin.manifest.contributes;
  const list =
    target.kind === "feature" ? c.features : target.kind === "command" ? c.commands : target.kind === "import" ? c.importers : c.exporters;
  return list.find((item) => item.id === target.contributionId) ?? null;
}

/** Parameters form for a plugin operation, command, importer or exporter. */
export function PluginRunDialog({
  disabled,
  onRun,
}: {
  disabled: boolean;
  onRun: (target: RunTarget, values: Record<string, FieldValue>) => void;
}) {
  const target = usePluginUi((state) => state.run);
  const close = usePluginUi((state) => state.openRun);
  const contribution = target ? contributionOf(target) : null;
  const [values, setValues] = useState<Record<string, FieldValue>>({});
  useEffect(() => {
    if (contribution) setValues(initialValues(contribution.params));
  }, [contribution]);
  if (!target || !contribution) return null;
  const translations = target.plugin.translations;
  const name = pluginText(translations, target.plugin.manifest.name);
  const action =
    target.kind === "import"
      ? t("Choose file…")
      : target.kind === "export"
        ? t("Export…")
        : target.kind === "command"
          ? t("Run")
          : t("Build");
  return (
    <Modal
      open
      onClose={() => close(null)}
      title={pluginText(translations, contribution.title)}
      description={t("Plugin {{value0}} {{value1}}", { value0: name, value1: target.plugin.active })}
    >
      <PluginForm fields={contribution.params} values={values} onChange={setValues} translations={translations} disabled={disabled} />
      <div className="modal-actions">
        <Button onClick={() => close(null)}>{t("Cancel")}</Button>
        <Button
          className="primary"
          disabled={disabled || !formValid(contribution.params, values)}
          onClick={() => {
            close(null);
            onRun(target, values);
          }}
        >
          {action}
        </Button>
      </div>
    </Modal>
  );
}
