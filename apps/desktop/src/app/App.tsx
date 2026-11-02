import { applicationMenus } from "../features/main-menu/applicationMenus";
import { copyProjectForEditing } from "../features/project-access/projectAccessApi";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import {
  AlertCircle,
  Box,
  ChevronDown,
  CircleCheck,
  Download,
  Plus,
  Search,
  Settings2,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import * as THREE from "three";
import { version as appVersion } from "../../package.json";
import { Button, IconButton, Modal, Select } from "../components/ui";
import { Updates } from "../components/Updates";
import { AccountButton } from "../features/account/AccountButton";
import { ActivityPanel } from "../features/activity-log/ActivityPanel";
import { candidateSourceStatus } from "../features/agents/candidateSource";
import { agentPermissionPreview } from "../features/agents/permissionPreview";
import type { CandidateReviewData } from "../features/agents/review/CandidateReview";
import { AssistantPanel } from "../features/assistant/AssistantPanel";
import { CadTaskIndicator } from "../features/cad-tasks/CadTaskIndicator";
import { CommandPalette } from "../features/command-palette/CommandPalette";
import { prepareProjectImport } from "../features/import/projectImport";
import { MainMenu } from "../features/main-menu/MainMenu";
import {
  assertApplyBase,
  captureApply,
} from "../features/model-apply/capturedApply";
import { useCommittedScene } from "../features/model-apply/useCommittedScene";
import { useModelApply } from "../features/model-apply/useModelApply";
import { useRevisionEvents } from "../features/model-apply/useRevisionEvents";
import { Outliner } from "../features/outliner/Outliner";
import { PluginHost } from "../features/plugins/PluginHost";
import {
  pluginPaletteCommands,
  usePlugins,
} from "../features/plugins/usePluginCommands";
import { ProjectAccessNotice } from "../features/project-access/ProjectAccessNotice";
import { useProjectAccess } from "../features/project-access/useProjectAccess";
import { ProjectFilesPanel } from "../features/project-files/ProjectFilesPanel";
import { ModelHistoryControls } from "../features/project-history/ModelHistoryControls";
import { RevisionList } from "../features/project-history/RevisionList";
import { useHistoryAction } from "../features/project-history/useHistoryAction";
import { PropertiesPanel } from "../features/properties/PropertiesPanel";
import { SelectionModelProperties } from "../features/reference-measurements/SelectionModelProperties";
import { StatusBar } from "../features/status-bar/StatusBar";
import { bodyLabel } from "../features/viewer/bodyLabel";
import { edgeLengthMm, edgeRadiusMm } from "../features/viewer/edgeSelection";
import { exportBodies } from "../features/viewer/exportBodies";
import { faceAreaMm2 } from "../features/viewer/faceSelection";
import { ModelViewport } from "../features/viewer/ModelViewport";
import { WorkspaceShell } from "../features/workspace-areas/WorkspaceShell";
import { showWorkspaceEditor } from "../features/workspace-areas/editorNavigation";
import { ProjectTabs } from "../features/project-tabs/ProjectTabs";
import { CadTaskPanel } from "../features/cad-tasks/CadTaskPanel";
import {
  errorText,
  fixedNumber,
  getLocale,
  rawError,
  systemText,
  t,
  useLocale,
} from "../i18n";
import {
  acknowledgeRecovery,
  cancelTask,
  confirmationSettings,
  convertStep,
  deleteProject,
  exportProjectBundle,
  exportStep,
  health,
  importProjectBundle,
  interruptedSessions,
  listProjects,
  native,
  needsConfirmation,
  plan,
  readProjectFile,
  requestNativePermission,
  resolveNativePermission,
  saveMesh,
  saveProject,
  saveProjectThumbnail,
} from "../lib/api";
import { download, exportMesh, loadModel, readFile } from "../lib/files";
import { buildModel, disposeModel, inspectModel } from "../lib/model";
import { persist, usePersistentState } from "../lib/persistence";
import { workspacePaletteCommands } from "../features/command-palette/workspaceCommands";
import { renderThumbnail } from "../lib/thumbnail";
import { readTypedCadDocument } from "../lib/typedCadDocument";
import { currentParameters, useWorkspace } from "../stores/workspace";
import { type Parameters, type Project, type ProjectFile } from "../types";
import { Dashboard } from "./Dashboard";
import {
  ActionReviewDialog,
  type PendingAction,
} from "./dialogs/ActionReviewDialog";
import { ExportDialog } from "./dialogs/ExportDialog";
import { NewProjectDialog } from "./dialogs/NewProjectDialog";
import { ParametersDialog } from "./dialogs/ParametersDialog";
import { ProgramDialog } from "./dialogs/ProgramDialog";
import { SettingsDialog } from "./dialogs/SettingsDialog";
import { RevisionComparison } from "./RevisionComparison";
const stamp = () => new Date().toISOString();
const legacyMotionPrompt = (text: string) =>
  text.startsWith("Добавь анимацию движения к текущей сборке.");
export default function App() {
  useLocale();
  const nav = useNavigate();
  const { projectId } = useParams();
  const queryClient = useQueryClient();
  const applyModel = useModelApply();
  useRevisionEvents();
  const {
    project,
    setProject,
    update,
    revise,
    busy,
    setBusy,
    error,
    setError,
    selected,
    selectedFace,
    selectedEdge,
    setSelected,
  } = useWorkspace();
  const projectAccess = useProjectAccess(project?.id, setError);
  const editingBlocked = busy || !projectAccess.writable;
  const projects = useQuery({ queryKey: ["projects"], queryFn: listProjects });
  const recovery = useQuery({
    queryKey: ["recovery"],
    queryFn: interruptedSessions,
  });
  const agentEnvironment = useQuery({
    queryKey: ["health"],
    queryFn: health,
    enabled: native && !!project,
    staleTime: 60000,
  });
  const [modal, setModal] = useState<
    "new" | "settings" | "export" | "palette" | "parameters" | null
  >(null);
  const stateKey = `forma.ui.project.${projectId ?? "home"}`;
  const [search, setSearch] = usePersistentState("forma.ui.search", "");
  const [renameTarget, setRenameTarget] = useState<Project | null>(null);
  const [renameName, setRenameName] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<Project | null>(null);
  const [deleteName, setDeleteName] = useState("");
  const previewProjects = useRef<Project[]>([]);
  const previewRunning = useRef(false);
  const previewAttempted = useRef(new Set<string>());
  const [prompt, setPrompt] = usePersistentState(`${stateKey}.prompt`, "");
  const [pending, setPending] = useState<PendingAction | null>(null);
  const [attachments, setAttachments] = usePersistentState<ProjectFile[]>(
    `${stateKey}.attachments`,
    [],
  );
  const [notice, setNotice] = useState("");
  const plugins = usePlugins(modal === "palette");
  const [liveEvents, setLiveEvents] = useState<
    { kind: string; text: string; createdAt: string }[]
  >([]);
  const [chatBusy, setChatBusy] = useState(false);
  const liveRef = useRef<{ kind: string; text: string; createdAt: string }[]>(
    [],
  );
  const [exportInfo, setExportInfo] = useState<
    Project["exports"][number] | null
  >(null);
  const [lockedBodies, setLockedBodies] = usePersistentState<string[]>(
    `${stateKey}.lockedBodies`,
    [],
  );
  const [selectedFeatureId, setSelectedFeatureId] = useState<string | null>(
    null,
  );
  const [compare, setCompare] = usePersistentState<string | null>(
    `${stateKey}.compare`,
    null,
  );
  const fileInput = useRef<HTMLInputElement>(null);
  const chatInput = useRef<HTMLTextAreaElement>(null);
  const parameters = currentParameters(project);
  const revision = project?.revisions.find(
    (r) => r.id === project.currentRevision,
  );
  const committedScene = useCommittedScene(project, revision);
  const loadingModel = committedScene.loading;
  const emptyScene = useMemo(() => new THREE.Group(), []);
  const generatedFiles = new Set(
    project?.revisions
      .flatMap((r) => [r.preview, r.program ? r.source : undefined])
      .filter(Boolean),
  );
  const referenceFiles =
    project?.files.filter((f) => !generatedFiles.has(f.name)) ?? [];
  const chatMessages =
    project?.messages.filter(
      (message) => message.role !== "user" || !legacyMotionPrompt(message.text),
    ) ?? [];
  const geometryKey = JSON.stringify(parameters);
  const model = useMemo(
    () =>
      committedScene.hasFile
        ? new THREE.Group()
        : buildModel(JSON.parse(geometryKey) as Parameters),
    [geometryKey, committedScene.hasFile],
  );
  const object =
    committedScene.object ?? (committedScene.hasFile ? emptyScene : model);
  const displayedObject = useRef(object);
  displayedObject.current = object;
  const pluginScene = useCallback(() => displayedObject.current, []);
  const displayedRevision = committedScene.hasFile
    ? project?.revisions.find(
        (item) =>
          committedScene.object &&
          item.id === committedScene.displayed?.revisionId,
      )
    : revision;
  const retainedLabel =
    displayedRevision && displayedRevision.id !== revision?.id
      ? t("Displayed model and properties: revision {{value0}}.", {
          value0:
            project!.revisions.findIndex(
              (item) => item.id === displayedRevision.id,
            ) + 1,
        })
      : null;
  const selectedBody = selected ? object.getObjectByName(selected) : undefined;
  const selectedBodyLabel = selectedBody ? bodyLabel(selectedBody) : selected;
  const selectedLabel =
    selectedEdge && selectedEdge.revisionId === project?.currentRevision
      ? t("{{value0}} · CAD edge {{value1}}", {
          value0: selectedBodyLabel ?? selectedEdge.bodyId,
          value1: selectedEdge.edgeOrdinal,
        })
      : selectedFace && selectedFace.revisionId === project?.currentRevision
        ? t("{{value0}} · CAD face {{value1}}", {
            value0: selectedBodyLabel ?? selectedFace.bodyId,
            value1: selectedFace.faceOrdinal,
          })
        : selectedBodyLabel;
  const selectedFaceAreaMm2 =
    selectedFace &&
    selectedFace.revisionId === project?.currentRevision &&
    selectedBody instanceof THREE.Mesh
      ? faceAreaMm2(selectedBody, selectedFace.faceOrdinal)
      : null;
  const selectedEdgeLengthMm =
    selectedEdge &&
    selectedEdge.revisionId === project?.currentRevision &&
    selectedBody instanceof THREE.Mesh
      ? edgeLengthMm(selectedBody, selectedEdge.edgeOrdinal)
      : null;
  const selectedEdgeRadiusMm =
    selectedEdge &&
    selectedEdge.revisionId === project?.currentRevision &&
    selectedBody instanceof THREE.Mesh
      ? edgeRadiusMm(selectedBody, selectedEdge.edgeOrdinal)
      : null;
  const stats = useMemo(() => inspectModel(object), [object]);
  const [before, setBefore] = useState<THREE.Group | null>(null);
  const [hiddenBodies, setHiddenBodies] = usePersistentState<string[]>(
    `${stateKey}.hiddenBodies`,
    [],
  );
  const bodies = useMemo(() => {
    const result: THREE.Mesh[] = [];
    object.traverse((node) => {
      if (node instanceof THREE.Mesh) result.push(node);
    });
    return result;
  }, [object]);
  useEffect(() => {
    bodies.forEach((body, index) => {
      body.visible = !hiddenBodies.includes(`${index}:${body.name}`);
    });
  }, [bodies, hiddenBodies]);
  useEffect(() => {
    bodies.forEach((body, index) => {
      body.userData.formaSelectionLocked = lockedBodies.includes(
        `${index}:${body.name}`,
      );
    });
  }, [bodies, lockedBodies]);
  useEffect(() => {
    let active = true;
    setBefore(null);
    const r = project?.revisions.find((r) => r.id === compare);
    if (!r) return;
    const file = project?.files.find((f) => f.name === (r.preview ?? r.source));
    if (file) {
      void loadModel(file, project?.id)
        .then((loaded) => {
          if (active) setBefore(loaded);
          else disposeModel(loaded);
        })
        .catch((e) => {
          if (active) setError(errorText(e));
        });
    } else setBefore(buildModel(r.parameters));
    return () => {
      active = false;
    };
  }, [compare, project?.id]);
  useEffect(() => {
    if (!native || !project?.id) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void listen<{ projectId: string; kind: string; text: string }>(
      "agent://progress",
      ({ payload }) => {
        if (payload.projectId !== project.id || payload.kind === "message")
          return;
        const previous = liveRef.current.at(-1);
        if (previous?.kind === payload.kind && previous.text === payload.text)
          return;
        const next = [
          ...liveRef.current,
          { kind: payload.kind, text: payload.text, createdAt: stamp() },
        ].slice(-100);
        liveRef.current = next;
        setLiveEvents(next);
      },
    )
      .then((stop) => {
        if (cancelled) stop();
        else unlisten = stop;
      })
      .catch((e) => setError(errorText(e)));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [project?.id, setError]);
  useEffect(() => {
    if (projectId || !projects.data) return;
    previewProjects.current = projects.data;
    if (previewRunning.current) return;
    previewRunning.current = true;
    void (async () => {
      while (true) {
        const next = previewProjects.current.find((item) => {
          const key = `${item.id}:${item.currentRevision}`;
          return (
            item.currentRevision &&
            item.thumbnailRevision !== item.currentRevision &&
            !previewAttempted.current.has(key)
          );
        });
        if (!next || !next.currentRevision) break;
        previewAttempted.current.add(`${next.id}:${next.currentRevision}`);
        const revision = next.revisions.find(
          (item) => item.id === next.currentRevision,
        );
        const file = next.files.find(
          (item) => item.name === (revision?.preview ?? revision?.source),
        );
        let preview: THREE.Group | null = null;
        try {
          preview =
            file && /\.(stl|obj|glb|3mf)$/i.test(file.name)
              ? await loadModel(file, next.id)
              : buildModel(currentParameters(next));
          const thumbnail = renderThumbnail(preview);
          if (!thumbnail) continue;
          const saved = await saveProjectThumbnail(
            next.id,
            next.currentRevision,
            thumbnail,
          );
          queryClient.setQueryData<Project[]>(["projects"], (current) =>
            current?.map((item) => (item.id === saved.id ? saved : item)),
          );
        } catch (cause) {
          console.warn("Project thumbnail could not be created", cause);
        } finally {
          if (preview) disposeModel(preview);
        }
      }
    })().finally(() => {
      previewRunning.current = false;
    });
  }, [projectId, projects.data, queryClient]);
  useEffect(
    () => () => {
      if (before) disposeModel(before);
    },
    [before],
  );
  useEffect(() => {
    const activeProject = useWorkspace.getState().project;
    if (!projectId) {
      if (activeProject) setProject(null);
      return;
    }
    if (projects.data && activeProject?.id !== projectId) {
      const p = projects.data.find((p) => p.id === projectId);
      if (p) setProject(p);
      else nav("/");
    }
  }, [projectId, projects.data, setProject, nav]);
  useEffect(() => {
    if (notice) {
      const t = setTimeout(() => setNotice(""), 5000);
      return () => clearTimeout(t);
    }
  }, [notice]);
  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (e.code === "F3") {
        e.preventDefault();
        setModal("palette");
        return;
      }
      if (target.closest("input,textarea,[contenteditable=true]")) return;
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.code === "KeyK") {
        e.preventDefault();
        setModal("palette");
      } else if (e.code === "KeyN") {
        e.preventDefault();
        setModal("new");
      } else if (e.code === "KeyO") {
        e.preventDefault();
        fileInput.current?.click();
      } else if (e.code === "KeyS") {
        e.preventDefault();
        if (project)
          saveProject(project)
            .then(() => setNotice(t("Project saved")))
            .catch((e) => setError(errorText(e)));
      } else if (e.code === "KeyE" && e.shiftKey) {
        e.preventDefault();
        setModal("export");
      }
    };
    window.addEventListener("keydown", f);
    return () => window.removeEventListener("keydown", f);
  });
  function refresh() {
    void queryClient.invalidateQueries({ queryKey: ["projects"] });
    void queryClient.invalidateQueries({ queryKey: ["model-history"] });
  }
  const historyAction = useHistoryAction(ask, refresh);
  async function ask(
    title: string,
    description: string,
    detail: string,
    run: () => Promise<void>,
    action = "modify_project",
    targetProjectId = project?.id,
    review?: CandidateReviewData,
  ) {
    try {
      if (!review && !needsConfirmation(await confirmationSettings(), action)) {
        await run();
        refresh();
        return;
      }
      const id =
        native && targetProjectId && !review
          ? await requestNativePermission(targetProjectId, action, detail)
          : undefined;
      setPending({
        title,
        description,
        detail,
        run,
        id,
        review,
        targetProjectId,
      });
    } catch (e) {
      if (
        !targetProjectId ||
        useWorkspace.getState().project?.id === targetProjectId
      )
        setError(errorText(e));
    }
  }
  async function approve(allow: boolean) {
    const p = pending;
    if (!p) return;
    setPending(null);
    try {
      if (
        allow &&
        p.review &&
        candidateSourceStatus(
          useWorkspace.getState().project,
          p.review.base,
        ) !== "valid"
      )
        throw new Error(
          t("Модель изменилась во время ответа. Повторите запрос."),
        );
      // Review can take longer than a grant's five-minute lifetime. Request only at the decision.
      const permissionId =
        p.review &&
        native &&
        (!allow ||
          needsConfirmation(await confirmationSettings(), "modify_project"))
          ? await requestNativePermission(
              p.review.base.id,
              "modify_project",
              p.detail,
            )
          : p.id;
      if (permissionId) await resolveNativePermission(permissionId, allow);
      if (allow) await p.run();
      else setNotice(t("Action denied. No model changes were made."));
    } catch (e) {
      if (
        !p.targetProjectId ||
        useWorkspace.getState().project?.id === p.targetProjectId
      )
        setError(errorText(e));
    } finally {
      refresh();
    }
  }
  async function openProject(p: Project) {
    if (busy) return;
    setProject(p);
    nav(`/project/${p.id}`);
  }
  async function importStepIntoScene(p: Project, name: string) {
    await ask(
      t("Import STEP model"),
      t(
        "Run the local CAD converter and create a new revision with a 3D preview.",
      ),
      name,
      async () => {
        setBusy(true, t("Converting STEP"));
        try {
          setProject(await convertStep(p.id, name));
          refresh();
        } finally {
          setBusy(false);
        }
      },
      "convert_file",
      p.id,
    );
  }
  async function importFiles(files: FileList | null) {
    if (!files?.length) return;
    try {
      const importedFiles = await Promise.all(Array.from(files).map(readFile));
      if (!project) {
        setAttachments(importedFiles);
        setModal("new");
        return;
      }
      await ask(
        t("Import project files"),
        t("Copy these files into this project."),
        importedFiles
          .map((f) => `${f.name} · ${fixedNumber(f.size / 1024, 1)} ${t("KB")}`)
          .join("\n"),
        async () => {
          const p = useWorkspace.getState().project;
          if (!p) return;
          const imported = await prepareProjectImport(p, importedFiles);
          await update(imported.project);
          if (!imported.hasMesh) {
            setAttachments((a) => [...a, ...importedFiles]);
            if (native && imported.step)
              await importStepIntoScene(
                useWorkspace.getState().project ?? imported.project,
                imported.step.name,
              );
            else setNotice(t("Files added to project attachments."));
          }
        },
      );
    } catch (e) {
      setError(errorText(e));
    } finally {
      if (fileInput.current) fileInput.current.value = "";
    }
  }
  async function captureScreenshot(data: string) {
    const blob = await (await fetch(data)).blob();
    const transfer = new DataTransfer();
    transfer.items.add(
      new File([blob], `viewport-${crypto.randomUUID()}.png`, {
        type: "image/png",
      }),
    );
    await importFiles(transfer.files);
    chatInput.current?.focus();
  }
  async function send(request = prompt) {
    if (!project || editingBlocked || !request.trim()) return;
    const text = request.trim();
    const p = project;
    let agentPreview: { description: string; detail: string };
    try {
      agentPreview = await agentPermissionPreview(p, text);
    } catch (cause) {
      if (useWorkspace.getState().project?.id === p.id)
        setError(errorText(cause));
      return;
    }
    await ask(
      t("Connect to your AI agent"),
      agentPreview.description,
      agentPreview.detail,
      async () => {
        setPrompt("");
        liveRef.current = [];
        setLiveEvents([]);
        setChatBusy(true);
        setBusy(true, t("Connecting to CLI"));
        try {
          await update({
            ...p,
            messages: [
              ...p.messages,
              {
                id: crypto.randomUUID(),
                role: "user",
                text,
                createdAt: stamp(),
              },
            ],
          });
          const result = await plan(
            p,
            text,
            attachments.map((file) => file.name),
            selectedEdge?.revisionId === p.currentRevision
              ? selectedEdge
              : selectedFace?.revisionId === p.currentRevision
                ? selectedFace
                : null,
          );
          setAttachments([]);
          const latest = useWorkspace.getState().project;
          if (!latest || candidateSourceStatus(latest, p) === "projectChanged")
            throw new Error(t("Project closed during the request"));
          await update({
            ...latest,
            messages: [
              ...latest.messages,
              ...liveRef.current
                .filter((e) => e.kind !== "message" && e.kind !== "error")
                .map((e) => ({
                  id: crypto.randomUUID(),
                  role: "event" as const,
                  text: e.text,
                  createdAt: e.createdAt,
                })),
              {
                id: crypto.randomUUID(),
                role: result.program ? "event" : "assistant",
                text: result.program
                  ? t("Программа получена. Модель ещё не построена.")
                  : result.message,
                createdAt: stamp(),
              },
            ],
          });
          liveRef.current = [];
          setLiveEvents([]);
          setChatBusy(false);
          if (!result.program) return;
          setBusy(false);
          await ask(
            t("Построить модель"),
            result.message,
            result.program,
            async () => {
              setBusy(true, t("Построение и проверка геометрии"));
              try {
                const current = useWorkspace.getState().project;
                if (
                  !current ||
                  candidateSourceStatus(current, p) === "projectChanged"
                )
                  throw new Error(t("Project closed during the request"));
                if (candidateSourceStatus(current, p) !== "valid")
                  throw new Error(
                    t("Модель изменилась во время ответа. Повторите запрос."),
                  );
                await applyModel(captureApply(p, result.program!, text), {
                  reviewPlan: result.reviewPlan,
                  assistantMessage: result.message,
                });
              } catch (error) {
                const current = useWorkspace.getState().project;
                if (current?.id === p.id)
                  await update({
                    ...current,
                    messages: [
                      ...current.messages,
                      {
                        id: crypto.randomUUID(),
                        role: "error",
                        text: rawError(error),
                        createdAt: stamp(),
                      },
                    ],
                  });
              } finally {
                setBusy(false);
              }
            },
            "modify_project",
            p.id,
            native && readTypedCadDocument(result.program)
              ? {
                  base: p,
                  source: result.program,
                  reviewPlan: result.reviewPlan,
                }
              : undefined,
          );
        } catch (e) {
          const latest = useWorkspace.getState().project;
          if (latest?.id === p.id)
            await update({
              ...latest,
              messages: [
                ...latest.messages,
                ...liveRef.current
                  .filter((e) => e.kind !== "message" && e.kind !== "error")
                  .map((e) => ({
                    id: crypto.randomUUID(),
                    role: "event" as const,
                    text: e.text,
                    createdAt: e.createdAt,
                  })),
                {
                  id: crypto.randomUUID(),
                  role: "error",
                  text: rawError(e),
                  createdAt: stamp(),
                },
              ],
            });
        } finally {
          setChatBusy(false);
          setBusy(false);
          setLiveEvents([]);
          liveRef.current = [];
        }
      },
      "run_agent",
    );
  }
  async function openReferenceFile(file: ProjectFile) {
    if (!project) return;
    try {
      if (native && /\.(step|stp)$/i.test(file.name)) {
        await ask(
          t("Import STEP model"),
          t(
            "Run the local CAD converter and create a new revision with a 3D preview.",
          ),
          file.name,
          async () => {
            setBusy(true, t("Converting STEP"));
            try {
              setProject(await convertStep(project.id, file.name));
              refresh();
            } finally {
              setBusy(false);
            }
          },
          "convert_file",
        );
        return;
      }
      const data = file.data ?? (await readProjectFile(project.id, file.name));
      download(await (await fetch(data)).blob(), file.name);
    } catch (cause) {
      setError(errorText(cause));
    }
  }
  return (
    <div className="app-shell">
      <Modal
        open={!!recovery.data?.length}
        title={t("Восстановление сессии")}
        description={t(
          "Работа агента была прервана. Последняя сохранённая модель и история доступны.",
        )}
        onClose={() => {
          void Promise.all(
            (recovery.data ?? []).map((item) => acknowledgeRecovery(item.id)),
          )
            .then(() => recovery.refetch())
            .catch((e) => setError(errorText(e)));
        }}
      >
        {recovery.data?.map((item) => (
          <div className="recovery-row" key={item.id}>
            <span>
              {projects.data?.find((p) => p.id === item.projectId)?.name ??
                t("Проект")}
              <small>
                {new Date(item.createdAt).toLocaleString(getLocale())}
              </small>
            </span>
            <Button
              onClick={() => {
                void acknowledgeRecovery(item.id)
                  .then(async () => {
                    nav(`/project/${item.projectId}`);
                    await recovery.refetch();
                  })
                  .catch((e) => setError(errorText(e)));
              }}
            >
              {t("Открыть модель")}
            </Button>
          </div>
        ))}
      </Modal>
      <header className="topbar">
        <button
          className="brand"
          onClick={() => {
            if (!busy) {
              setProject(null);
              nav("/");
              refresh();
            }
          }}
        >
          <span className="brand-mark">
            <Box size={21} strokeWidth={1.7} />
          </span>
          forma
          <span className="brand-divider" />
        </button>
        {project ? (
          <div className="project-switch">
            <Select
              aria-label={t("Switch project")}
              value={project.id}
              onChange={(e) => {
                const next = projects.data?.find(
                  (p) => p.id === e.target.value,
                );
                if (next) void openProject(next);
              }}
              disabled={editingBlocked}
            >
              {(projects.data ?? [project]).map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </div>
        ) : (
          <span className="project-switch">{t("Your workspace")}</span>
        )}
        {project && (
          <span className="saved">
            <span className="status-dot" />
            {t("All changes saved")}
          </span>
        )}
        <div className="topbar-actions">
          {project && (
            <CadTaskIndicator projectId={project.id} onError={setError} />
          )}
          {project && (
            <ModelHistoryControls
              project={project}
              blocked={editingBlocked || !!pending || !!modal || loadingModel}
              onAction={historyAction}
            />
          )}
          <Updates blocked={busy || !!pending || !!modal || loadingModel} />
          <AccountButton />
          <button
            className="command-button"
            onClick={() => setModal("palette")}
          >
            <Search size={14} />
            <span>{t("Search anything")}</span>
            <kbd>Ctrl K</kbd>
          </button>
          <IconButton
            label={t("Settings")}
            onClick={() => setModal("settings")}
          >
            <Settings2 size={17} />
          </IconButton>
          {project ? (
            <Button
              className="primary"
              disabled={busy || !object.children.length}
              onClick={() => setModal("export")}
            >
              <Download size={14} />
              {t("Export")}
              <ChevronDown size={13} />
            </Button>
          ) : (
            <Button className="primary" onClick={() => setModal("new")}>
              <Plus size={15} />
              {t("New project")}
            </Button>
          )}
        </div>
      </header>
      <MainMenu
        groups={applicationMenus(
          {
            duplicate: () => {
              if (project)
                void ask(
                  t("Duplicate project"),
                  t(
                    "Create an independent project with the same files and history.",
                  ),
                  project.name,
                  async () => {
                    if (native) {
                      await openProject(
                        await copyProjectForEditing(project.id),
                      );
                      return;
                    }
                    await openProject(
                      await saveProject({
                        ...project,
                        id: crypto.randomUUID(),
                        name: t("{{value0}} copy", { value0: project.name }),
                        createdAt: stamp(),
                        updatedAt: stamp(),
                      }),
                    );
                  },
                );
            },
            create: () => setModal("new"),
            import: () => fileInput.current?.click(),
            export: () => setModal("export"),
            parameters: () => setModal("parameters"),
            palette: () => setModal("palette"),
            settings: (page) => {
              if (page) persist("forma.ui.settingsPage", JSON.stringify(page));
              setModal("settings");
            },
            workspaces: () =>
              document
                .querySelector<HTMLElement>(
                  '[role="tablist"][aria-label="' +
                    t("Workspaces") +
                    '"] button',
                )
                ?.focus(),
          },
          !!project,
          editingBlocked,
        )}
      />
      {project && (
        <ProjectAccessNotice
          projectId={project.id}
          {...projectAccess}
          onCopy={(copy) => {
            refresh();
            void openProject(copy);
          }}
          onError={setError}
        />
      )}
      {project && (
        <ProjectTabs
          projects={projects.data ?? [project]}
          activeId={project.id}
          blocked={busy}
          onSelect={(item) => void openProject(item)}
          onHome={() => {
            setProject(null);
            nav("/");
          }}
        />
      )}
      {!project ? (
        <Dashboard
          projects={projects.data ?? []}
          loading={projects.isLoading}
          error={projects.error}
          search={search}
          onSearch={setSearch}
          onSettings={() => setModal("settings")}
          onImport={() => fileInput.current?.click()}
          onImportBundle={() =>
            void importProjectBundle()
              .then((item) => {
                if (item) {
                  refresh();
                  setNotice(t("Project bundle imported"));
                  void openProject(item);
                }
              })
              .catch((cause) => setError(errorText(cause)))
          }
          onExportBundle={(item, redactConversation) =>
            void ask(
              t("Export project bundle"),
              t("Save a portable copy of this project."),
              item.name,
              async () => {
                const path = await exportProjectBundle(
                  item.id,
                  redactConversation,
                );
                if (path) setNotice(t("Project bundle exported"));
              },
              "export_file",
              item.id,
            )
          }
          onNew={() => setModal("new")}
          onOpen={(item) => void openProject(item)}
          onRename={(item) => {
            setRenameTarget(item);
            setRenameName(item.name);
          }}
          onDelete={(item) => {
            setDeleteTarget(item);
            setDeleteName("");
          }}
          onRefresh={refresh}
          onError={setError}
        />
      ) : (
        <div
          className="workspace workspace-area-container"
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault();
            if (!busy) void importFiles(event.dataTransfer.files);
          }}
        >
          <WorkspaceShell
            key={project.id}
            projectId={project.id}
            slots={{
              viewport3d: (
                <ModelViewport
                  projectId={project.id}
                  object={object}
                  before={before}
                  interactive={
                    !committedScene.hasFile || committedScene.interactive
                  }
                  targetRevision={revision?.id ?? ""}
                  displayedRevision={committedScene.displayed?.revisionId ?? ""}
                  phase={committedScene.phase}
                  loading={loadingModel}
                  failed={committedScene.failed}
                  failure={errorText(committedScene.cause)}
                  retainedLabel={retainedLabel}
                  onRetry={committedScene.retry}
                  onScreenshot={(data) => void captureScreenshot(data)}
                />
              ),
              outliner: (
                <Outliner
                  project={project}
                  bodies={bodies}
                  source={displayedRevision?.program ?? revision?.program ?? ""}
                  selected={selected}
                  featureId={selectedFeatureId}
                  hidden={hiddenBodies}
                  locked={lockedBodies}
                  onSelect={(id) => {
                    setSelected(id);
                    setSelectedFeatureId(null);
                  }}
                  onFeature={setSelectedFeatureId}
                  onVisibility={(key) =>
                    setHiddenBodies((current) =>
                      current.includes(key)
                        ? current.filter((value) => value !== key)
                        : [...current, key],
                    )
                  }
                  onLock={(key) =>
                    setLockedBodies((current) =>
                      current.includes(key)
                        ? current.filter((value) => value !== key)
                        : [...current, key],
                    )
                  }
                  onEdit={() => setModal("parameters")}
                />
              ),
              files: (
                <ProjectFilesPanel
                  exports={project.exports}
                  onExport={setExportInfo}
                  files={referenceFiles}
                  revision={revision}
                  blocked={editingBlocked}
                  onImport={() => fileInput.current?.click()}
                  onEdit={() => setModal("parameters")}
                  onOpen={(file) => void openReferenceFile(file)}
                />
              ),
              parameters: (
                <PropertiesPanel
                  source={displayedRevision?.program ?? revision?.program ?? ""}
                  featureId={null}
                  onEdit={() => setModal("parameters")}
                  initialTab="parameters"
                >
                  {null}
                </PropertiesPanel>
              ),
              properties: (
                <PropertiesPanel
                  source={displayedRevision?.program ?? revision?.program ?? ""}
                  featureId={selectedFeatureId}
                  onEdit={() => setModal("parameters")}
                >
                  <SelectionModelProperties
                    selectedBodyId={selected}
                    sceneToken={committedScene.selectionScene.token}
                    interactive={committedScene.interactive && !compare}
                    hostAvailable={
                      native &&
                      !!agentEnvironment.data?.some(
                        (item) =>
                          item.name === "OpenCascade kernel" && item.available,
                      )
                    }
                    project={project}
                    revision={displayedRevision}
                    selectedLabel={selectedLabel ?? null}
                    selectedFace={selectedFace}
                    selectedFaceAreaMm2={selectedFaceAreaMm2}
                    selectedEdge={selectedEdge}
                    selectedEdgeLengthMm={selectedEdgeLengthMm}
                    selectedEdgeRadiusMm={selectedEdgeRadiusMm}
                    stats={stats}
                    thickness={
                      displayedRevision?.parameters.thickness ??
                      parameters.thickness
                    }
                    onEditParameters={() => setModal("parameters")}
                  />
                  {compare &&
                    project.currentRevision &&
                    compare !== project.currentRevision && (
                      <RevisionComparison
                        project={project}
                        fromRevisionId={compare}
                        toRevisionId={project.currentRevision}
                      />
                    )}
                </PropertiesPanel>
              ),
              history: (
                <RevisionList
                  project={project}
                  blocked={editingBlocked}
                  compare={compare}
                  onCompare={setCompare}
                  onRestore={(item) =>
                    void ask(
                      t("Restore this revision"),
                      t(
                        "The current history will be preserved. Restoration creates a new revision.",
                      ),
                      item.prompt,
                      () =>
                        revise(
                          item.parameters,
                          t("Restored revision {{value0}}", {
                            value0: project.revisions.indexOf(item) + 1,
                          }),
                          item.source,
                          item.preview,
                          item.program,
                          item.programBase,
                        ),
                    )
                  }
                />
              ),
              ai: (
                <AssistantPanel
                  project={project}
                  messages={chatMessages}
                  events={liveEvents}
                  busy={chatBusy}
                  blocked={editingBlocked}
                  stage={useWorkspace.getState().stage}
                  prompt={prompt}
                  onPrompt={setPrompt}
                  inputRef={chatInput}
                  attachments={attachments}
                  selectedLabel={selectedLabel ?? null}
                  status={
                    !native
                      ? t("Desktop only")
                      : liveEvents.some((event) => event.kind === "connected")
                        ? t("Connected")
                        : agentEnvironment.isFetching
                          ? t("Checking…")
                          : agentEnvironment.data?.find(
                                (item) =>
                                  item.name ===
                                  (project.agent === "codex"
                                    ? "OpenAI Codex"
                                    : project.agent === "claude"
                                      ? "Claude Code"
                                      : "Custom CLI"),
                              )?.available
                            ? t("Installed")
                            : t("Not found")
                  }
                  onSend={() => void send()}
                  onAttach={() => fileInput.current?.click()}
                  onRemoveAttachment={(index) =>
                    setAttachments((current) =>
                      current.filter((_, i) => i !== index),
                    )
                  }
                  onClearSelection={() => setSelected(null)}
                  onSettings={() => setModal("settings")}
                  onFresh={() =>
                    void ask(
                      t("Start a fresh conversation"),
                      t(
                        "Clear chat messages while keeping models and revisions.",
                      ),
                      project.name,
                      () => update({ ...project, messages: [] }),
                    )
                  }
                  onCancel={() =>
                    void cancelTask(project.id).catch((cause) =>
                      setError(errorText(cause)),
                    )
                  }
                  onNotice={setNotice}
                  onError={setError}
                />
              ),
              tasks: <CadTaskPanel onError={setError} />,
              activity: <ActivityPanel messages={project.messages} />,
              logs: <ActivityPanel messages={project.messages} errorsOnly />,
            }}
          />
        </div>
      )}
      <StatusBar
        busy={busy}
        chatBusy={chatBusy}
        stage={useWorkspace.getState().stage}
        native={native}
        revisions={project?.revisions.length ?? null}
        units={project?.units}
        version={appVersion}
      />
      <input
        ref={fileInput}
        type="file"
        multiple
        hidden
        accept=".stl,.obj,.glb,.3mf,.step,.stp,.dxf,.png,.jpg,.jpeg,.webp,.pdf"
        onChange={(e) => void importFiles(e.target.files)}
      />
      {error && (
        <div className="toast error-toast" role="alert">
          <AlertCircle size={17} />
          <span>{errorText(error)}</span>
          <button
            aria-label={t("Dismiss error")}
            onClick={() => setError(null)}
          >
            <X size={16} />
          </button>
        </div>
      )}
      {notice && (
        <div className="toast" role="status">
          <CircleCheck size={17} />
          {systemText(notice)}
        </div>
      )}
      <ActionReviewDialog
        pending={pending}
        onDecision={(allow) => void approve(allow)}
      />
      <Modal
        open={!!exportInfo}
        onClose={() => setExportInfo(null)}
        title={t("Export details")}
        description={exportInfo?.name ?? ""}
      >
        <p>
          {exportInfo &&
            new Date(exportInfo.createdAt).toLocaleString(getLocale())}
        </p>
        <p className="field-hint">
          {t("Saved to the destination chosen during export.")}
        </p>
        <Button
          onClick={() => {
            void navigator.clipboard
              .writeText(exportInfo?.name ?? "")
              .then(() => setNotice(t("Filename copied")))
              .catch((e) => setError(errorText(e)));
          }}
        >
          {t("Copy filename")}
        </Button>
      </Modal>
      <Modal
        open={!!renameTarget}
        onClose={() => setRenameTarget(null)}
        title={t("Rename project")}
        description={t(
          "The model and its history will keep their current files.",
        )}
      >
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (!renameTarget || !renameName.trim()) return;
            const latest =
              (queryClient.getQueryData<Project[]>(["projects"]) ?? []).find(
                (item) => item.id === renameTarget.id,
              ) ?? renameTarget;
            void saveProject({
              ...latest,
              name: renameName.trim(),
              updatedAt: stamp(),
            })
              .then(() => {
                setRenameTarget(null);
                refresh();
                setNotice(t("Project renamed"));
              })
              .catch((cause) => setError(errorText(cause)));
          }}
        >
          <label>
            {t("Project name")}
            <input
              autoFocus
              value={renameName}
              maxLength={80}
              onChange={(event) => setRenameName(event.target.value)}
            />
          </label>
          <div className="modal-actions">
            <Button type="button" onClick={() => setRenameTarget(null)}>
              {t("Cancel")}
            </Button>
            <Button
              className="primary"
              type="submit"
              disabled={!renameName.trim()}
            >
              {t("Save name")}
            </Button>
          </div>
        </form>
      </Modal>
      <Modal
        open={!!deleteTarget}
        onClose={() => setDeleteTarget(null)}
        title={t("Delete project")}
        description={t(
          "This permanently deletes the project, its revisions, attachments and stored files.",
        )}
      >
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (!deleteTarget || deleteName !== deleteTarget.name) return;
            void deleteProject(deleteTarget.id)
              .then(() => {
                queryClient.setQueryData<Project[]>(["projects"], (current) =>
                  current?.filter((item) => item.id !== deleteTarget.id),
                );
                setDeleteTarget(null);
                refresh();
                setNotice(t("Project deleted"));
              })
              .catch((cause) => setError(errorText(cause)));
          }}
        >
          <label>
            {t("delete.confirmBefore")}{" "}
            <strong className="delete-project-name">
              {deleteTarget?.name}
            </strong>
            {t("delete.confirmAfter")}
            <input
              autoFocus
              autoComplete="off"
              value={deleteName}
              onChange={(event) => setDeleteName(event.target.value)}
            />
          </label>
          <div className="modal-actions">
            <Button type="button" onClick={() => setDeleteTarget(null)}>
              {t("Cancel")}
            </Button>
            <Button
              className="danger"
              type="submit"
              disabled={deleteName !== deleteTarget?.name}
            >
              <Trash2 size={14} />
              {t("Delete permanently")}
            </Button>
          </div>
        </form>
      </Modal>
      <NewProjectDialog
        importNames={attachments.map((file) => file.name)}
        open={modal === "new"}
        close={() => setModal(null)}
        onCreate={async (p) => {
          try {
            if (attachments.length)
              p = (await prepareProjectImport(p, attachments)).project;
            const saved = await saveProject(p);
            await openProject(saved);
            setModal(null);
            setAttachments([]);
            refresh();
            const step =
              native && attachments.find((f) => /\.st(e)?p$/i.test(f.name));
            if (step) await importStepIntoScene(saved, step.name);
          } catch (e) {
            setError(errorText(e));
          }
        }}
      />
      <SettingsDialog
        open={modal === "settings"}
        close={() => setModal(null)}
        project={project}
        onAgent={async (agent) => {
          if (project && !busy) await update({ ...project, agent });
        }}
      />
      {!revision || revision.source || revision.program ? (
        <ProgramDialog
          open={modal === "parameters"}
          close={() => setModal(null)}
          source={revision?.program ?? ""}
          importedSource={revision?.source}
          disabled={editingBlocked}
          nativeCadAvailable={
            agentEnvironment.data?.some(
              (item) => item.name === "OpenCascade kernel" && item.available,
            ) ?? false
          }
          nativeCadChecked={!!agentEnvironment.data}
          onApply={(program) => {
            const base = useWorkspace.getState().project;
            if (!base) return;
            const captured = captureApply(
              base,
              program,
              t("Изменён исходный код модели"),
            );
            setModal(null);
            void ask(
              t("Построить модель"),
              t("Проверить геометрию и сохранить ревизию."),
              program,
              async () => {
                assertApplyBase(useWorkspace.getState().project, captured);
                setBusy(true, t("Построение геометрии"));
                try {
                  await applyModel(captured);
                } finally {
                  setBusy(false);
                }
              },
              "modify_project",
              base.id,
            );
          }}
        />
      ) : (
        <ParametersDialog
          open={modal === "parameters"}
          close={() => setModal(null)}
          parameters={parameters}
          disabled={editingBlocked}
          onApply={(p) => {
            setModal(null);
            void ask(
              t("Update model parameters"),
              t("Create a new revision with these dimensions."),
              t(
                "{{value0}}: {{value1}} × {{value2}} × {{value3}} mm\nWall: {{value4}} mm\nHoles: {{value5}} × Ø{{value6}} mm",
                {
                  value0: t(p.kind),
                  value1: p.width,
                  value2: p.depth,
                  value3: p.height,
                  value4: p.thickness,
                  value5: p.holes,
                  value6: p.holeDiameter,
                },
              ),
              () => revise(p, t("Updated model parameters")),
            );
          }}
        />
      )}
      <ExportDialog
        open={modal === "export"}
        close={() => setModal(null)}
        hasModel={!!object.children.length}
        bodies={exportBodies(revision?.program)}
        onExport={async (format, bodyId) => {
          setModal(null);
          await ask(
            t("Export model"),
            t("Write the current model to an export file."),
            t("{{value0}} · {{value1}} · millimeters", {
              value0: project?.name ?? "",
              value1: format.toUpperCase(),
            }),
            async () => {
              if (!project) return;
              const name = `${project.name.replace(/[^\p{L}\p{N}_-]/gu, "-")}-r${project.revisions.length}.${format}`;
              if (format === "step") {
                if (revision?.source && !/\.st(e)?p$/i.test(revision.source))
                  throw new Error(
                    t("STEP export of imported meshes is not supported."),
                  );
                const path = await exportStep(project.id, parameters, bodyId);
                if (!path) {
                  setNotice(t("Export cancelled"));
                  return;
                }
                setNotice(t("STEP exported to {{value0}}", { value0: path }));
              } else {
                const blob = await exportMesh(object, format, bodyId);
                if (native) {
                  const path = await saveMesh(project.id, name, blob);
                  if (!path) {
                    setNotice(t("Export cancelled"));
                    return;
                  }
                } else download(blob, name);
                setNotice(t("Exported {{value0}}", { value0: name }));
              }
              await update({
                ...project,
                exports: [...project.exports, { name, createdAt: stamp() }],
              });
            },
            "export_file",
          );
        }}
      />
      <CommandPalette
        onSettingsPage={(page) => {
          persist("forma.ui.settingsPage", JSON.stringify(page));
          setModal("settings");
        }}
        activeProject={project}
        onEntity={(kind, id) => {
          if (!project) return;
          if (kind === "body") {
            setSelected(id);
            setSelectedFeatureId(null);
            showWorkspaceEditor(project.id, "outliner");
          } else if (kind === "feature") {
            setSelectedFeatureId(id);
            showWorkspaceEditor(project.id, "properties");
          } else if (kind === "parameter")
            showWorkspaceEditor(project.id, "parameters");
          else {
            setCompare(id === project.currentRevision ? null : id);
            showWorkspaceEditor(project.id, "history");
          }
        }}
        open={modal === "palette"}
        close={() => setModal(null)}
        search={search}
        onSearch={setSearch}
        projects={projects.data ?? []}
        onProject={(item) => {
          void openProject(item);
          setModal(null);
        }}
        onNew={() => setModal("new")}
        onImport={() => {
          setModal(null);
          fileInput.current?.click();
        }}
        onExport={() => setModal("export")}
        onParameters={() => setModal("parameters")}
        onAsk={() => {
          setModal(null);
          if (project) showWorkspaceEditor(project.id, "ai");
          requestAnimationFrame(() => chatInput.current?.focus());
        }}
        onSettings={() => setModal("settings")}
        extraCommands={[
          ...pluginPaletteCommands(plugins.data ?? [], () => setModal(null)),
          ...workspacePaletteCommands(project?.id),
        ]}
      />
      <PluginHost
        project={project}
        disabled={editingBlocked}
        ask={ask}
        notify={setNotice}
        fail={setError}
        scene={pluginScene}
      />
    </div>
  );
}
