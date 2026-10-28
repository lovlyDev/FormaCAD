import { useState } from "react";
import type * as THREE from "three";
import { t, number } from "../../i18n";
import { Button } from "../../components/ui";
import { PluginToolbarMenu } from "../plugins/PluginToolbarMenu";
import "./viewportSidebar.css";
export function ViewportSidebar({
  object,
  selected,
  onFit,
  onReset,
}: {
  object: THREE.Group;
  selected: string | null;
  onFit: () => void;
  onReset: () => void;
}) {
  const [tab, setTab] = useState("item");
  const node = selected ? object.getObjectByName(selected) : null;
  return (
    <aside className="viewport-sidebar" aria-label={t("Viewport sidebar")}>
      <div role="tablist" aria-label={t("Viewport sidebar tabs")}>
        {["item", "view", "plugins"].map((value) => (
          <button
            key={value}
            role="tab"
            aria-selected={tab === value}
            onClick={() => setTab(value)}
          >
            {value === "item"
              ? t("Item")
              : value === "view"
                ? t("View")
                : t("Plugins")}
          </button>
        ))}
      </div>
      {tab === "item" ? (
        <div className="workspace-content-list">
          {node ? (
            <>
              <strong>{node.name}</strong>
              <p>
                {t(
                  "Display transform; exact CAD properties are in Properties.",
                )}
              </p>
              <dl>
                {(["x", "y", "z"] as const).map((axis) => (
                  <div key={axis}>
                    <dt>
                      {t("Position {{value0}}", { value0: axis.toUpperCase() })}
                    </dt>
                    <dd>
                      {number(node.position[axis], 3)} {t("mm")}
                    </dd>
                  </div>
                ))}
              </dl>
            </>
          ) : (
            <p>{t("No selection")}</p>
          )}
        </div>
      ) : tab === "view" ? (
        <div className="workspace-content-list">
          <Button onClick={onFit}>{t("Fit model")}</Button>
          <Button onClick={onReset}>{t("Reset camera")}</Button>
        </div>
      ) : (
        <div className="workspace-content-list">
          <PluginToolbarMenu />
          <p>
            {t("Enabled plugin actions use the existing permission checks.")}
          </p>
        </div>
      )}
    </aside>
  );
}
