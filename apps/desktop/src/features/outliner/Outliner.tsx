import { useCallback, useMemo, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import type * as THREE from "three";
import {
  Box,
  Eye,
  EyeOff,
  Lock,
  Unlock,
  SlidersHorizontal,
} from "lucide-react";
import { IconButton } from "../../components/ui";
import { t } from "../../i18n";
import { bodyLabel } from "../viewer/bodyLabel";
import {
  readTypedCadDocument,
  featureDependencies,
  type TypedCadFeature,
} from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { BodyGroups } from "../body-groups/BodyGroups";
import type { GroupCommand } from "../body-groups/bodyGroupCommands";
import { FeatureCommandMenu, type FeatureMenuTarget } from "../activity-log/FeatureCommandMenu";
import "./outliner.css";
export function Outliner({
  project,
  bodies,
  source,
  selected,
  featureId,
  hidden,
  locked,
  onSelect,
  onFeature,
  onVisibility,
  onLock,
  onEdit,
  onGroupCommand,
}: {
  project: Project;
  bodies: THREE.Object3D[];
  source: string;
  selected: string | null;
  featureId: string | null;
  hidden: string[];
  locked: string[];
  onSelect: (id: string) => void;
  onFeature: (id: string) => void;
  onVisibility: (key: string) => void;
  onLock: (key: string) => void;
  onEdit: () => void;
  onGroupCommand?: (command: GroupCommand) => void;
}) {
  const [query, setQuery] = useState("");
  const [menu, setMenu] = useState<FeatureMenuTarget | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);
  const openMenu = (event: MouseEvent<HTMLButtonElement> | KeyboardEvent<HTMLButtonElement>, feature: TypedCadFeature) => {
    if ("key" in event && event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
    event.preventDefault();
    event.stopPropagation();
    const anchor = event.currentTarget;
    const rect = anchor.getBoundingClientRect();
    setMenu({
      anchor, feature,
      x: "clientX" in event ? event.clientX : rect.left,
      y: "clientY" in event ? event.clientY : rect.bottom,
    });
  };
  const document = useMemo(() => readTypedCadDocument(source), [source]);
  const matches = (...values: string[]) =>
    values.some((value) =>
      value.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
    );
  return (
    <div className="outliner">
      {menu && <FeatureCommandMenu target={menu} close={closeMenu} />}
      <label className="outliner-search">
        <input
          aria-label={t("Search model tree")}
          placeholder={t("Search model tree")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>
      {document && (onGroupCommand || document.groups?.length) ? (
        <BodyGroups groups={document.groups ?? []} onCommand={onGroupCommand}
          selectable={bodies.filter((body) => body.visible &&
            !hidden.includes(body.name) && !locked.includes(body.name)).map((body) => body.name)} />
      ) : null}
      <div role="tree" aria-label={t("Model tree")}>
        <div
          role="treeitem"
          aria-expanded="true"
          aria-level={1}
          className="outliner-project"
        >
          <Box size={14} />
          <strong>{project.name}</strong>
          <IconButton label={t("Edit model parameters")} onClick={onEdit}>
            <SlidersHorizontal size={13} />
          </IconButton>
        </div>
        <div role="group">
          {bodies.map((body, index) => {
            const key = `${index}:${body.name}`,
              label =
                bodyLabel(body) || t("Body {{value0}}", { value0: index + 1 });
            const definition = document?.bodies.find(
              (item) => item.id === body.name || item.name === label,
            );
            const ids = new Set<string>();
            const visit = (id: string) => {
              if (ids.has(id)) return;
              ids.add(id);
              const feature = document?.features.find((item) => item.id === id);
              if (feature) featureDependencies(feature).forEach(visit);
            };
            if (definition) visit(definition.sourceFeatureId);
            const features =
              document?.features.filter((item) => ids.has(item.id)) ?? [];
            if (
              !matches(
                label,
                body.name,
                ...features.flatMap((item) => [
                  item.id,
                  item.name,
                  item.operation.type,
                ]),
              )
            )
              return null;
            return (
              <div
                key={key}
                role="treeitem"
                aria-level={2}
                aria-expanded={features.length ? true : undefined}
                aria-selected={selected === body.name}
              >
                <div className="outliner-row">
                  <button
                    className={selected === body.name ? "is-selected" : ""}
                    disabled={locked.includes(key)}
                    onClick={() => onSelect(body.name)}
                  >
                    <Box size={14} />
                    <span>{label}</span>
                  </button>
                  <IconButton
                    label={
                      hidden.includes(key)
                        ? t("Show {{value0}}", { value0: label })
                        : t("Hide {{value0}}", { value0: label })
                    }
                    onClick={() => onVisibility(key)}
                  >
                    {hidden.includes(key) ? (
                      <EyeOff size={13} />
                    ) : (
                      <Eye size={13} />
                    )}
                  </IconButton>
                  <IconButton
                    label={
                      locked.includes(key)
                        ? t("Unlock selection {{value0}}", { value0: label })
                        : t("Lock selection {{value0}}", { value0: label })
                    }
                    onClick={() => onLock(key)}
                  >
                    {locked.includes(key) ? (
                      <Lock size={13} />
                    ) : (
                      <Unlock size={13} />
                    )}
                  </IconButton>
                </div>
                {!!features.length && (
                  <div role="group">
                    {features
                      .filter((feature) =>
                        matches(
                          label,
                          feature.name,
                          feature.id,
                          feature.operation.type,
                        ),
                      )
                      .map((feature) => (
                        <button
                          role="treeitem"
                          aria-level={3}
                          aria-selected={featureId === feature.id}
                          className={`outliner-feature ${featureId === feature.id ? "is-selected" : ""}`}
                          key={feature.id}
                          onClick={() => onFeature(feature.id)}
                          onDoubleClick={onEdit}
                          onContextMenu={(event) => openMenu(event, feature)}
                          onKeyDown={(event) => openMenu(event, feature)}
                        >
                          <span>{feature.name}</span>
                          <small>{t(feature.operation.type)}</small>
                        </button>
                      ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
      {bodies.length === 0 && (
        <p className="field-hint">{t("No geometry yet")}</p>
      )}
      {document && (
        <div className="outliner-unassigned">
          <h4>{t("Features")}</h4>
          {document.features
            .filter((feature) => matches(feature.id, feature.name))
            .map((feature) => (
              <button
                key={feature.id}
                onClick={() => onFeature(feature.id)}
                onDoubleClick={onEdit}
                onContextMenu={(event) => openMenu(event, feature)}
                onKeyDown={(event) => openMenu(event, feature)}
              >
                {feature.name}
                <small>{t(feature.operation.type)}</small>
              </button>
            ))}
        </div>
      )}
    </div>
  );
}
