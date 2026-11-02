import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { t } from "../../i18n";
import { useWorkspaceLayout } from "../../stores/layout/useWorkspaceLayout";
import { WorkspaceTabs } from "../workspaces/WorkspaceTabs";
import { AreaHeader } from "./AreaHeader";
import { AreaSeparator } from "./AreaSeparator";
import {
  areas,
  changeEditor,
  editorTypes,
  joinArea,
  resizeSplit,
  splitArea,
  type EditorType,
} from "./layoutModel";
import { minimumSize, placeLayout } from "./layoutGeometry";
import { editorLabels } from "./editorLabels";
import {
  editorNavigationEvent,
  type EditorNavigation,
} from "./editorNavigation";
import { useInterfacePreferences } from "../settings/WorkspaceSettings";
import "./workspaceAreas.css";
export function WorkspaceShell({
  projectId,
  slots,
}: {
  projectId: string;
  slots: Record<EditorType, ReactNode>;
}) {
  useInterfacePreferences();
  const workspace = useWorkspaceLayout(projectId);
  const stage = useRef<HTMLDivElement>(null),
    mounted = useRef(new Set<EditorType>());
  const [size, setSize] = useState({ width: 1000, height: 700 });
  const [focusId, setFocusId] = useState<string | null>(null),
    [maximized, setMaximized] = useState<string | null>(null);
  const root = workspace.active.layout.root;
  const minSize = minimumSize(root);
  const geometry = placeLayout(
    root,
    Math.max(size.width, minSize.width),
    Math.max(size.height, minSize.height),
  );
  const visible = new Map(
    geometry.areas.map((item) => [item.area.editor, item]),
  );
  visible.forEach((_, type) => mounted.current.add(type));
  useLayoutEffect(() => {
    if (!stage.current) return;
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0].contentRect;
      setSize({ width: rect.width, height: rect.height });
    });
    observer.observe(stage.current);
    return () => observer.disconnect();
  }, []);
  const change = (next: typeof root) =>
    workspace.change({ layoutVersion: 1, root: next });
  const maximize = (id: string) =>
    setMaximized((current) => (current === id ? null : id));
  const focusArea = (id: string) => {
    setFocusId(id);
    stage.current
      ?.querySelector<HTMLElement>(`[data-area-id="${id}"]`)
      ?.focus();
  };
  useEffect(() => {
    const navigate = (event: Event) => {
      const detail = (event as CustomEvent<EditorNavigation>).detail;
      if (
        !detail ||
        detail.projectId !== projectId ||
        !editorTypes.includes(detail.editor)
      )
        return;
      setMaximized(null);
      const existing = areas(root).find(
        (area) => area.editor === detail.editor,
      );
      if (existing) {
        focusArea(existing.id);
        return;
      }
      const target =
        areas(root).find(
          (area) => area.id === focusId && area.editor !== "viewport3d",
        ) ??
        areas(root).find((area) => area.editor === "ai") ??
        areas(root)[0];
      change(changeEditor(root, target.id, detail.editor));
      requestAnimationFrame(() => focusArea(target.id));
    };
    window.addEventListener(editorNavigationEvent, navigate);
    return () => window.removeEventListener(editorNavigationEvent, navigate);
  });
  return (
    <div className="workspace-shell">
      <WorkspaceTabs
        {...workspace}
        onActivate={(id) => {
          workspace.activate(id);
          setMaximized(null);
        }}
        onCreate={workspace.create}
        onRename={workspace.rename}
        onReset={() => {
          workspace.reset();
          setMaximized(null);
        }}
      />
      <div className="workspace-stage-scroll">
        <div
          className="workspace-stage"
          ref={stage}
          onKeyDown={(event) => {
            const target = event.target as HTMLElement;
            if (
              target.closest(
                "input, textarea, select, [contenteditable=true], [role=combobox], [role=dialog]",
              )
            )
              return;
            const current =
              target.closest<HTMLElement>("[data-area-id]")?.dataset.areaId ??
              focusId;
            if (event.ctrlKey && event.code === "Space" && current) {
              event.preventDefault();
              maximize(current);
            }
            if (
              event.code === "F6" ||
              (event.ctrlKey && event.code === "Tab")
            ) {
              event.preventDefault();
              const candidates = geometry.areas.filter(
                (item) => !maximized || item.area.id === maximized,
              );
              const index = candidates.findIndex(
                (item) => item.area.id === current,
              );
              const next =
                candidates[
                  (index + (event.shiftKey ? -1 : 1) + candidates.length) %
                    candidates.length
                ];
              if (next) focusArea(next.area.id);
            }
          }}
          style={{ minWidth: minSize.width, minHeight: minSize.height }}
        >
          {editorTypes
            .filter((type) => mounted.current.has(type))
            .map((type) => {
              const placed = visible.get(type),
                area = placed?.area;
              const hidden = !placed || (!!maximized && area?.id !== maximized);
              return (
                <section
                  key={`${projectId}:${type}`}
                  className="workspace-editor"
                  data-editor={type}
                  data-area-id={area?.id}
                  tabIndex={-1}
                  aria-label={t(editorLabels[type])}
                  hidden={hidden}
                  onPointerDownCapture={(event) => {
                    if (
                      !(event.target as HTMLElement).closest(
                        "button,input,textarea,select,[contenteditable=true],[role=combobox]",
                      )
                    )
                      event.currentTarget.focus({ preventScroll: true });
                  }}
                  onFocusCapture={() => {
                    if (area) setFocusId(area.id);
                  }}
                  style={
                    hidden
                      ? { display: "none" }
                      : maximized
                        ? {
                            left: 0,
                            top: 0,
                            width: size.width,
                            height: size.height,
                            zIndex: 2,
                          }
                        : {
                            left: placed!.x,
                            top: placed!.y,
                            width: placed!.width,
                            height: placed!.height,
                          }
                  }
                >
                  {area && (
                    <AreaHeader
                      area={area}
                      maximized={!!maximized}
                      canSplitX={
                        !maximized &&
                        geometry.areas.length < editorTypes.length &&
                        placed!.width >= 366
                      }
                      canSplitY={
                        !maximized &&
                        geometry.areas.length < editorTypes.length &&
                        placed!.height >= 246
                      }
                      canJoin={!maximized && geometry.areas.length > 1}
                      onEditor={(editor) =>
                        change(changeEditor(root, area.id, editor))
                      }
                      onSplit={(axis) =>
                        change(
                          splitArea(root, area.id, axis, crypto.randomUUID()),
                        )
                      }
                      onJoin={() => change(joinArea(root, area.id))}
                      onMaximize={() => maximize(area.id)}
                    />
                  )}
                  <div className="workspace-editor-content">{slots[type]}</div>
                </section>
              );
            })}
          {!maximized &&
            geometry.separators.map((split) => (
              <AreaSeparator
                key={split.node.id}
                split={split}
                onResize={(id, ratio) => change(resizeSplit(root, id, ratio))}
              />
            ))}
        </div>
      </div>
      <nav className="workspace-bottom-tabs" aria-label={t("Workspace panels")}>
        {(["history", "tasks", "activity", "logs"] as EditorType[]).map(
          (type) => (
            <button
              key={type}
              aria-pressed={visible.has(type)}
              onClick={() => {
                setMaximized(null);
                const existing = areas(root).find(
                  (area) => area.editor === type,
                );
                if (existing) {
                  change(joinArea(root, existing.id));
                  return;
                }
                const target =
                    areas(root).find((area) => area.editor === "viewport3d") ??
                    areas(root)[0],
                  id = crypto.randomUUID();
                change(
                  changeEditor(splitArea(root, target.id, "y", id), id, type),
                );
              }}
            >
              {t(editorLabels[type])}
            </button>
          ),
        )}
      </nav>
    </div>
  );
}
