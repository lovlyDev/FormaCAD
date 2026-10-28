import { useEffect } from "react";
import { Checkbox, Select } from "../../components/ui";
import { t } from "../../i18n";
import { usePersistentState } from "../../lib/persistence";
import { useNavigationPreferences } from "./workspacePreferences";
export function useInterfacePreferences() {
  const [density, setDensity] = usePersistentState(
    "forma.ui.interface.density",
    "standard",
  );
  const [scale, setScale] = usePersistentState(
    "forma.ui.interface.textScale",
    1,
  );
  useEffect(() => {
    document.documentElement.dataset.density = [
      "compact",
      "standard",
      "large",
    ].includes(density)
      ? density
      : "standard";
    document.documentElement.style.setProperty(
      "--workspace-text-scale",
      String([1, 1.15, 1.3].includes(scale) ? scale : 1),
    );
  }, [density, scale]);
  return { density, setDensity, scale, setScale };
}
export function WorkspaceAppearance() {
  const { density, setDensity, scale, setScale } = useInterfacePreferences();
  return (
    <>
      <label>
        {t("Interface density")}
        <Select
          aria-label={t("Interface density")}
          value={density}
          onChange={(event) => setDensity(event.target.value)}
        >
          <option value="compact">{t("Compact")}</option>
          <option value="standard">{t("Standard")}</option>
          <option value="large">{t("Large")}</option>
        </Select>
      </label>
      <label>
        {t("Interface text size")}
        <Select
          aria-label={t("Interface text size")}
          value={String(scale)}
          onChange={(event) => setScale(Number(event.target.value))}
        >
          <option value="1">{t("Standard")}</option>
          <option value="1.15">{t("Larger")}</option>
          <option value="1.3">{t("Largest")}</option>
        </Select>
      </label>
    </>
  );
}
export function WorkspaceInputSettings() {
  const preferences = useNavigationPreferences();
  return (
    <div className="workspace-content-list">
      <h3>{t("Input")}</h3>
      <Checkbox
        checked={preferences.emulateNumpad}
        onChange={preferences.setEmulateNumpad}
      >
        {t("Emulate numpad with number keys")}
      </Checkbox>
      <p>
        {t(
          "Shortcuts use physical keys and work with Russian and English layouts. Text fields keep their own shortcuts.",
        )}
      </p>
      <p>
        {t("F3: command search · F6: next area · Ctrl+Space: maximize area")}
      </p>
    </div>
  );
}
export function WorkspaceNavigationSettings() {
  const preferences = useNavigationPreferences();
  return (
    <div className="workspace-content-list">
      <h3>{t("Navigation")}</h3>
      <Checkbox
        checked={preferences.orbitSelection}
        onChange={preferences.setOrbitSelection}
      >
        {t("Orbit around selection")}
      </Checkbox>
      <Checkbox
        checked={preferences.zoomCursor}
        onChange={preferences.setZoomCursor}
      >
        {t("Zoom toward pointer")}
      </Checkbox>
      <label>
        {t("Zoom sensitivity")}
        <input
          aria-label={t("Zoom sensitivity")}
          type="range"
          min="0.2"
          max="3"
          step="0.1"
          value={preferences.zoomSpeed}
          onChange={(event) =>
            preferences.setZoomSpeed(Number(event.target.value))
          }
        />
      </label>
      <label>
        {t("Pan sensitivity")}
        <input
          aria-label={t("Pan sensitivity")}
          type="range"
          min="0.1"
          max="2"
          step="0.1"
          value={preferences.panSpeed}
          onChange={(event) =>
            preferences.setPanSpeed(Number(event.target.value))
          }
        />
      </label>
    </div>
  );
}
