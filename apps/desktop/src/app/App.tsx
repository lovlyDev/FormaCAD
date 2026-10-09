import { candidateSourceStatus } from "../features/agents/candidateSource";
import { captureApply, assertApplyBase } from "../features/model-apply/capturedApply";
import { useModelApply } from "../features/model-apply/useModelApply";
import { useCommittedScene } from "../features/model-apply/useCommittedScene";
import { useRevisionEvents } from "../features/model-apply/useRevisionEvents";
import { ModelHistoryControls } from "../features/project-history/ModelHistoryControls";
import { useProjectAccess } from "../features/project-access/useProjectAccess";
import { ProjectAccessNotice } from "../features/project-access/ProjectAccessNotice";
import { CadTaskIndicator } from "../features/cad-tasks/CadTaskIndicator";
import { useHistoryAction } from "../features/project-history/useHistoryAction";
import { revisionHistoryLabel } from "../features/project-history/revisionHistoryLabel";
import { ActionReviewDialog, type PendingAction } from "./dialogs/ActionReviewDialog";
import type { CandidateReviewData } from "../features/agents/review/CandidateReview";
import { readTypedCadDocument } from "../lib/typedCadDocument";
import { prepareProjectImport } from "../features/import/projectImport";
import { CommandPalette } from "./dialogs/CommandPalette";
import { Dashboard } from "./Dashboard";
import { SelectionModelProperties } from "../features/reference-measurements/SelectionModelProperties";
import { NewProjectDialog } from "./dialogs/NewProjectDialog";
import { SettingsDialog } from "./dialogs/SettingsDialog";
import { ParametersDialog } from "./dialogs/ParametersDialog";
import { ExportDialog } from "./dialogs/ExportDialog";
import { ProgramDialog } from "./dialogs/ProgramDialog";
import { MessageText } from "../features/agents/MessageText";
import { RevisionComparison } from "./RevisionComparison";
import { agentPermissionPreview } from "../features/agents/permissionPreview";
import {
  t,
  useLocale,
  quantity,
  fixedNumber,
  getLocale,
  systemText,
  errorText,
  rawError,
} from "../i18n";
import { usePersistentState } from "../lib/persistence";
import { Updates } from "../components/Updates";
import { version as appVersion } from "../../package.json";
import { listen } from "@tauri-apps/api/event";
import {
  lazy,
  Suspense,
  useEffect,
  useMemo,
  useRef,
  useState,
  Component,
  type ReactNode,
  type ErrorInfo,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "react-router-dom";
import { motion } from "framer-motion";
import * as THREE from "three";
import {
  ArrowUp,
  ArrowUpRight,
  Box,
  ChevronDown,
  CircleCheck,
  Command,
  Download,
  FileBox,
  FileImage,
  Folder,
  History,
  LoaderCircle,
  PanelLeftClose,
  Paperclip,
  Plus,
  Search,
  Settings2,
  Sparkles,
  Square,
  Trash2,
  Undo2,
  X,
  SlidersHorizontal,
  Copy,
  AlertCircle,
  GitBranch,
  Eye,
  EyeOff,
} from "lucide-react";
import {
  Button,
  IconButton,
  Modal,
  Select,
  TreeFolder,
} from "../components/ui";
import {
  type Project,
  type Parameters,
  type ProjectFile,
} from "../types";
import { useWorkspace, currentParameters } from "../stores/workspace";
import {
  readProjectFile,
  interruptedSessions,
  acknowledgeRecovery,
  convertStep,
  confirmationSettings,
  needsConfirmation,
  listProjects,
  saveProject,
  exportProjectBundle,
  importProjectBundle,
  saveProjectThumbnail,
  deleteProject,
  health,
  plan,
  native,
  cancelTask,
  exportStep,
  saveMesh,
  requestNativePermission,
  resolveNativePermission,
} from "../lib/api";
import { buildModel, inspectModel, disposeModel } from "../lib/model";
import { readFile, loadModel, exportMesh, download } from "../lib/files";
import { renderThumbnail } from "../lib/thumbnail";
import { bodyLabel } from "../features/viewer/bodyLabel";
import { faceAreaMm2 } from "../features/viewer/faceSelection";
import { edgeLengthMm, edgeRadiusMm } from "../features/viewer/edgeSelection";
import { exportBodies } from "../features/viewer/exportBodies";
const Viewer = lazy(() => import("../features/viewer/Viewer"));
class ViewerBoundary extends Component<
  { children: ReactNode },
  { error: boolean }
> {
  state = { error: false };
  static getDerivedStateFromError() {
    return { error: true };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Viewer failed", error.message, info.componentStack);
  }
  render() {
    return this.state.error ? (
      <div className="empty-state">
        <AlertCircle />
        <h3>{t("3D view is unavailable")}</h3>
        <p>{t("Check that hardware acceleration and WebGL 2 are enabled.")}</p>
      </div>
    ) : (
      this.props.children
    );
  }
}
const stamp = () => new Date().toISOString();
const legacyMotionPrompt = (text: string) =>
  text.startsWith("Добавь анимацию движения к текущей сборке.");
const revisionPrompt = (text: string) =>
  revisionHistoryLabel(text) ?? (legacyMotionPrompt(text) ? t("Настроено движение сборки") : text);
const date = (v: string) =>
  new Date(v).toLocaleDateString(getLocale(), {
    month: "short",
    day: "numeric",
  });
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
  const [tab, setTab] = usePersistentState<"files" | "history">(
    `${stateKey}.tab`,
    "files",
  );
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
  const [leftOpen, setLeftOpen] = usePersistentState("forma.ui.leftOpen", true);
  const [leftWidth, setLeftWidth] = usePersistentState(
    "forma.ui.leftWidth",
    238,
  );
  const [rightWidth, setRightWidth] = usePersistentState(
    "forma.ui.rightWidth",
    350,
  );
  const [compare, setCompare] = usePersistentState<string | null>(
    `${stateKey}.compare`,
    null,
  );
  const fileInput = useRef<HTMLInputElement>(null);
  const chatInput = useRef<HTMLTextAreaElement>(null);
  const end = useRef<HTMLDivElement>(null);
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
    () => committedScene.hasFile ? new THREE.Group() : buildModel(JSON.parse(geometryKey) as Parameters),
    [geometryKey, committedScene.hasFile],
  );
  const object = committedScene.object ?? (committedScene.hasFile ? emptyScene : model);
  const displayedRevision = committedScene.hasFile
    ? project?.revisions.find(item => committedScene.object && item.id === committedScene.displayed?.revisionId)
    : revision;
  const retainedLabel = displayedRevision && displayedRevision.id !== revision?.id
    ? t("Displayed model and properties: revision {{value0}}.", { value0: project!.revisions.findIndex(item => item.id === displayedRevision.id) + 1 })
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
    end.current?.scrollIntoView({ behavior: "smooth" });
  }, [liveEvents]);
  useEffect(() => () => disposeModel(model), [model]);
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
    end.current?.scrollIntoView({ behavior: "smooth" });
  }, [project?.messages.length, busy]);
  useEffect(() => {
    if (notice) {
      const t = setTimeout(() => setNotice(""), 5000);
      return () => clearTimeout(t);
    }
  }, [notice]);
  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.key === "k") {
        e.preventDefault();
        setModal("palette");
      } else if (e.key === "n") {
        e.preventDefault();
        setModal("new");
      } else if (e.key === "o") {
        e.preventDefault();
        fileInput.current?.click();
      } else if (e.key === "s") {
        e.preventDefault();
        if (project)
          saveProject(project)
            .then(() => setNotice(t("Project saved")))
            .catch((e) => setError(errorText(e)));
      } else if (e.key === "E" || (e.key === "e" && e.shiftKey)) {
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
      setPending({ title, description, detail, run, id, review, targetProjectId });
    } catch (e) {
      if (!targetProjectId || useWorkspace.getState().project?.id === targetProjectId) setError(errorText(e));
    }
  }
  async function approve(allow: boolean) {
    const p = pending;
    if (!p) return;
    setPending(null);
    try {
      if (allow && p.review && candidateSourceStatus(useWorkspace.getState().project, p.review.base) !== "valid")
        throw new Error(t("Модель изменилась во время ответа. Повторите запрос."));
      // Review can take longer than a grant's five-minute lifetime. Request only at the decision.
      const permissionId = p.review && native && (!allow || needsConfirmation(await confirmationSettings(), "modify_project"))
        ? await requestNativePermission(p.review.base.id, "modify_project", p.detail)
        : p.id;
      if (permissionId) await resolveNativePermission(permissionId, allow);
      if (allow) await p.run();
      else setNotice(t("Action denied. No model changes were made."));
    } catch (e) {
      if (!p.targetProjectId || useWorkspace.getState().project?.id === p.targetProjectId) setError(errorText(e));
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
    await ask(t("Import STEP model"), t("Run the local CAD converter and create a new revision with a 3D preview."), name, async () => {
      setBusy(true, t("Converting STEP"));
      try { setProject(await convertStep(p.id, name)); refresh(); }
      finally { setBusy(false); }
    }, "convert_file", p.id);
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
            if (native && imported.step) await importStepIntoScene(useWorkspace.getState().project ?? imported.project, imported.step.name);
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
      if (useWorkspace.getState().project?.id === p.id) setError(errorText(cause));
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
          if (!latest || candidateSourceStatus(latest, p) === "projectChanged") throw new Error(t("Project closed during the request"));
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
                if (!current || candidateSourceStatus(current, p) === "projectChanged")
                  throw new Error(t("Project closed during the request"));
                if (candidateSourceStatus(current, p) !== "valid")
                  throw new Error(
                    t("Модель изменилась во время ответа. Повторите запрос."),
                  );
                await applyModel(captureApply(p, result.program!, text), { reviewPlan: result.reviewPlan, assistantMessage: result.message });
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
            native && readTypedCadDocument(result.program) ? { base: p, source: result.program, reviewPlan: result.reviewPlan } : undefined,
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
  const resize = (side: "left" | "right", event: React.PointerEvent) => {
    const start = event.clientX,
      initial = side === "left" ? leftWidth : rightWidth;
    event.currentTarget.setPointerCapture(event.pointerId);
    const target = event.currentTarget;
    const move = (e: Event) => {
      const x = (e as PointerEvent).clientX;
      const value = initial + (side === "left" ? x - start : start - x);
      if (side === "left") setLeftWidth(Math.max(195, Math.min(340, value)));
      else setRightWidth(Math.max(300, Math.min(540, value)));
    };
    const stop = () => {
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", stop);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", stop);
  };
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
          {project && <CadTaskIndicator projectId={project.id} onError={setError} />}
          {project && <ModelHistoryControls project={project} blocked={editingBlocked || !!pending || !!modal || loadingModel} onAction={historyAction} />}
          <Updates blocked={busy || !!pending || !!modal || loadingModel} />
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
      {project && <ProjectAccessNotice projectId={project.id} {...projectAccess} onCopy={copy => { refresh(); void openProject(copy); }} onError={setError} />}
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
                const path = await exportProjectBundle(item.id, redactConversation);
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
          className="workspace"
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            if (!busy) void importFiles(e.dataTransfer.files);
          }}
        >
          {leftOpen && (
            <>
              <aside className="left-panel" style={{ width: leftWidth }}>
                <div className="panel-heading">
                  <span>{t("PROJECT")}</span>
                  <IconButton
                    label={t("Collapse project panel")}
                    onClick={() => setLeftOpen(false)}
                  >
                    <PanelLeftClose size={15} />
                  </IconButton>
                </div>
                <div className="project-title">
                  <span className="project-symbol">
                    <Box size={20} />
                  </span>
                  <div>
                    <strong>{project.name}</strong>
                    <small>
                      {project.agent === "codex"
                        ? "Codex"
                        : project.agent === "claude"
                          ? "Claude Code"
                          : t("Custom agent")}{" "}
                      <span>·</span> {t(project.units)}
                    </small>
                  </div>
                  <IconButton
                    label={t("Duplicate project")}
                    disabled={editingBlocked}
                    onClick={() =>
                      void ask(
                        t("Duplicate project"),
                        t(
                          "Create an independent project with the same files and history.",
                        ),
                        project.name,
                        async () => {
                          const duplicate = {
                            ...project,
                            id: crypto.randomUUID(),
                            name: t("{{value0}} copy", {
                              value0: project.name,
                            }),
                            createdAt: stamp(),
                            updatedAt: stamp(),
                          };
                          await openProject(await saveProject(duplicate));
                        },
                      )
                    }
                  >
                    <Copy size={13} />
                  </IconButton>
                </div>
                <div className="panel-tabs">
                  <button
                    className={tab === "files" ? "active" : ""}
                    onClick={() => setTab("files")}
                  >
                    <Folder size={14} />
                    {t("Files")}
                  </button>
                  <button
                    className={tab === "history" ? "active" : ""}
                    onClick={() => setTab("history")}
                  >
                    <History size={14} />
                    {t("History")}
                    <span>{project.revisions.length}</span>
                  </button>
                </div>
                <div className="panel-scroll">
                  {tab === "files" ? (
                    <>
                      <div className="tree-heading">
                        <span>{t("MODEL TREE")}</span>
                        <IconButton
                          label={t("Edit model parameters")}
                          disabled={editingBlocked}
                          onClick={() => setModal("parameters")}
                        >
                          <SlidersHorizontal size={13} />
                        </IconButton>
                      </div>
                      <TreeFolder
                        folderId="assembly"
                        title={t("Assembly")}
                        count={stats.bodies}
                      >
                        {bodies.map((o, i) => (
                          <div
                            key={o.uuid}
                            className={`tree-row body-row ${selected === o.name ? "selected" : ""}`}
                          >
                            <button
                              className="body-select"
                              onClick={() => setSelected(o.name)}
                            >
                              <Box size={14} />
                              {bodyLabel(o) ||
                                t("Body {{value0}}", { value0: i + 1 })}
                            </button>
                            <IconButton
                              label={`${hiddenBodies.includes(`${i}:${o.name}`) ? t("Show") : t("Hide")} ${bodyLabel(o) || t("Body {{value0}}", { value0: i + 1 })}`}
                              onClick={() =>
                                setHiddenBodies((current) => {
                                  const next = new Set(current);
                                  if (next.has(`${i}:${o.name}`))
                                    next.delete(`${i}:${o.name}`);
                                  else next.add(`${i}:${o.name}`);
                                  return [...next];
                                })
                              }
                            >
                              {hiddenBodies.includes(`${i}:${o.name}`) ? (
                                <EyeOff size={13} />
                              ) : (
                                <Eye size={13} />
                              )}
                            </IconButton>
                          </div>
                        ))}
                      </TreeFolder>
                      <div className="tree-heading files-heading">
                        <span>{t("PROJECT FILES")}</span>
                        <IconButton
                          label={t("Import files")}
                          disabled={editingBlocked}
                          onClick={() => fileInput.current?.click()}
                        >
                          <Plus size={14} />
                        </IconButton>
                      </div>
                      <TreeFolder folderId="workspace" title={t("workspace")}>
                        {revision && (
                          <button
                            className="tree-row file-row"
                            onClick={() => setModal("parameters")}
                          >
                            <FileBox size={14} />
                            {revision.program
                              ? revision.program.trimStart().startsWith("{")
                                ? "model.cad.json"
                                : "model.py"
                              : "model.parameters.json"}
                          </button>
                        )}
                      </TreeFolder>
                      <TreeFolder
                        folderId="attachments"
                        title={t("attachments")}
                        count={referenceFiles.length}
                      >
                        {referenceFiles.map((f) => (
                          <button
                            className="tree-row file-row"
                            key={f.name}
                            onClick={async () => {
                              try {
                                if (native && /\.(step|stp)$/i.test(f.name)) {
                                  await ask(
                                    t("Import STEP model"),
                                    t(
                                      "Run the local CAD converter and create a new revision with a 3D preview.",
                                    ),
                                    f.name,
                                    async () => {
                                      setBusy(true, t("Converting STEP"));
                                      try {
                                        setProject(
                                          await convertStep(project.id, f.name),
                                        );
                                        refresh();
                                      } finally {
                                        setBusy(false);
                                      }
                                    },
                                    "convert_file",
                                  );
                                  return;
                                }
                                const data =
                                  f.data ??
                                  (await readProjectFile(project.id, f.name));
                                download(
                                  await (await fetch(data)).blob(),
                                  f.name,
                                );
                              } catch (error) {
                                setError(String(error));
                              }
                            }}
                          >
                            {f.kind === "image" ? (
                              <FileImage size={14} />
                            ) : (
                              <FileBox size={14} />
                            )}
                            <span className="truncate">{f.name}</span>
                          </button>
                        ))}
                      </TreeFolder>
                      <TreeFolder
                        folderId="revisions"
                        title={t("Revisions")}
                        count={project.revisions.length}
                        initial={false}
                      >
                        {project.revisions.map((r, i) => (
                          <button
                            className="tree-row file-row"
                            key={r.id}
                            onClick={() => {
                              setTab("history");
                              requestAnimationFrame(() =>
                                document
                                  .getElementById(`revision-${r.id}`)
                                  ?.scrollIntoView({
                                    behavior: "smooth",
                                    block: "center",
                                  }),
                              );
                            }}
                          >
                            <History size={13} />
                            <span>
                              {t("Revision")} {i + 1}
                            </span>
                          </button>
                        ))}
                      </TreeFolder>
                      <TreeFolder
                        folderId="exports"
                        title={t("exports")}
                        count={project.exports.length}
                      >
                        {project.exports.map((e, i) => (
                          <button
                            className="tree-row file-row"
                            key={`${e.name}-${i}`}
                            onClick={() => setExportInfo(e)}
                          >
                            <Download size={13} />
                            <span className="truncate">{e.name}</span>
                          </button>
                        ))}
                      </TreeFolder>
                    </>
                  ) : (
                    <div className="revision-list">
                      {[...project.revisions].reverse().map((r, i) => (
                        <div
                          id={`revision-${r.id}`}
                          className={`revision-item ${r.id === project.currentRevision ? "current" : ""}`}
                          key={r.id}
                        >
                          <div>
                            <GitBranch size={14} />
                            <strong>
                              {t("Revision")} {project.revisions.length - i}
                            </strong>
                            {r.id === project.currentRevision && (
                              <span>{t("Current")}</span>
                            )}
                          </div>
                          <p>{revisionPrompt(r.prompt)}</p>
                          <small>
                            {date(r.createdAt)} ·{" "}
                            {new Date(r.createdAt).toLocaleTimeString(
                              getLocale(),
                              {
                                hour: "2-digit",
                                minute: "2-digit",
                              },
                            )}
                          </small>
                          <div className="revision-actions">
                            <button
                              disabled={
                                busy || r.id === project.currentRevision
                              }
                              onClick={() =>
                                void ask(
                                  t("Restore this revision"),
                                  t(
                                    "The current history will be preserved. Restoration creates a new revision.",
                                  ),
                                  r.prompt,
                                  () =>
                                    revise(
                                      r.parameters,
                                      t("Restored revision {{value0}}", {
                                        value0:
                                          project.revisions.indexOf(r) + 1,
                                      }),
                                      r.source,
                                      r.preview,
                                      r.program,
                                      r.programBase,
                                    ),
                                )
                              }
                            >
                              <Undo2 size={12} />
                              {t("Restore")}
                            </button>
                            <button
                              disabled={editingBlocked}
                              onClick={() =>
                                setCompare(compare === r.id ? null : r.id)
                              }
                            >
                              {compare === r.id
                                ? t("Close comparison")
                                : t("Compare")}
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
                <SelectionModelProperties
                  selectedBodyId={selected}
                  sceneToken={committedScene.selectionScene.token}
                  interactive={committedScene.interactive && !compare}
                  hostAvailable={native && !!agentEnvironment.data?.some(item => item.name === "OpenCascade kernel" && item.available)}
                  project={project}
                  revision={displayedRevision}
                  selectedLabel={selectedLabel ?? null}
                  selectedFace={selectedFace}
                  selectedFaceAreaMm2={selectedFaceAreaMm2}
                  selectedEdge={selectedEdge}
                  selectedEdgeLengthMm={selectedEdgeLengthMm}
                  selectedEdgeRadiusMm={selectedEdgeRadiusMm}
                  stats={stats}
                  thickness={displayedRevision?.parameters.thickness ?? parameters.thickness}
                  onEditParameters={() => setModal("parameters")}
                />
                {compare && project.currentRevision && compare !== project.currentRevision &&
                  <RevisionComparison project={project} fromRevisionId={compare} toRevisionId={project.currentRevision} />}
              </aside>
              <div
                role="separator"
                aria-label={t("Resize project panel")}
                aria-orientation="vertical"
                className="resize-handle"
                onPointerDown={(e) => resize("left", e)}
              />
            </>
          )}
          <section className="center-panel">
            <div className="document-tabs">
              {!leftOpen && (
                <IconButton
                  label={t("Open project panel")}
                  onClick={() => setLeftOpen(true)}
                >
                  <Folder size={15} />
                </IconButton>
              )}
              <div className="document-tab">
                <Box size={14} />
                {revision?.program
                  ? t("Model")
                  : (revision?.source ?? t("Model"))}
                <span className="tab-dot" />
              </div>
              {compare && (
                <button onClick={() => setCompare(null)}>
                  {t("Comparison")}
                  <X size={13} />
                </button>
              )}
              <span className="document-tab-space" />
              <span className="revision-badge">
                {project.revisions.length
                  ? t("Revision {{value0}}", {
                      value0:
                        project.revisions.findIndex(
                          (r) => r.id === project.currentRevision,
                        ) + 1,
                    })
                  : t("No revisions")}
              </span>
            </div>
            <div className={`viewer-area ${before ? "comparison" : ""}`} data-displayed-revision={committedScene.displayed?.revisionId ?? ""} data-target-revision={revision?.id ?? ""} data-scene-phase={committedScene.phase}>
              <ViewerBoundary key={project.id}>
                <Suspense
                  fallback={
                    <div className="empty-state">
                      <LoaderCircle className="spin" />
                      {t("Loading 3D workspace…")}
                    </div>
                  }
                >
                  {before && (
                    <Viewer
                      object={before}
                      comparison
                      onScreenshot={(data) => void captureScreenshot(data)}
                    />
                  )}
                  <Viewer
                    object={object}
                    interactive={!committedScene.hasFile || committedScene.interactive}
                    onScreenshot={(data) => void captureScreenshot(data)}
                  />
                </Suspense>
              </ViewerBoundary>
              {loadingModel && (
                <div className="model-loading">
                  <LoaderCircle className="spin" />
                  {t("Reading geometry…")}
                  {retainedLabel && <small>{retainedLabel}</small>}
                </div>
              )}
              {committedScene.failed && <div className="model-loading" role="alert"><AlertCircle size={16} /><span>{t("The committed preview could not be loaded. The previous valid model is shown when available.")}</span>{retainedLabel && <small>{retainedLabel}</small>}<small>{errorText(committedScene.cause)}</small><Button onClick={committedScene.retry}>{t("Retry preview loading")}</Button></div>}
            </div>
            <div className="model-bottom">
              <span>
                <Box size={12} />
                {quantity("bodies", stats.bodies)}
              </span>
              <span>
                {selected
                  ? t("{{value0}} selected", {
                      value0: selectedLabel ?? selected,
                    })
                  : t("No selection")}
              </span>
            </div>
          </section>
          <div
            role="separator"
            aria-label={t("Resize assistant panel")}
            aria-orientation="vertical"
            className="resize-handle"
            onPointerDown={(e) => resize("right", e)}
          />
          <aside className="chat-panel" style={{ width: rightWidth }}>
            <div className="chat-header">
              <span>
                <Sparkles size={16} />
                {t("Assistant")}
              </span>
              <div>
                <IconButton
                  label={t("Start a fresh conversation")}
                  disabled={busy || !chatMessages.length}
                  onClick={() =>
                    void ask(
                      t("Start a fresh conversation"),
                      t(
                        "Clear chat messages while keeping models and revisions.",
                      ),
                      project.name,
                      () => update({ ...project, messages: [] }),
                    )
                  }
                >
                  <Plus size={16} />
                </IconButton>
              </div>
            </div>
            <button
              className="agent-picker"
              onClick={() => setModal("settings")}
            >
              <span className="agent-logo">
                <Command size={15} />
              </span>
              {project.agent === "codex"
                ? "OpenAI Codex"
                : project.agent === "claude"
                  ? "Claude Code"
                  : t("Custom agent")}
              <span className="agent-local">
                {!native
                  ? t("Desktop only")
                  : liveEvents.some((e) => e.kind === "connected")
                    ? t("Connected")
                    : agentEnvironment.isFetching
                      ? t("Checking…")
                      : agentEnvironment.data?.find(
                            (h) =>
                              h.name ===
                              (project.agent === "codex"
                                ? "OpenAI Codex"
                                : "Claude Code"),
                          )?.available
                        ? t("Installed")
                        : t("Not found")}
              </span>
              <ChevronDown size={12} />
            </button>
            <div className="chat-scroll">
              {chatMessages.length === 0 ? (
                <>
                  <div className="assistant-welcome">
                    <div className="welcome-icon">
                      <Sparkles size={24} />
                    </div>
                    <h2>{t("What will we make?")}</h2>
                    <p>
                      {t("Describe a part, explore an idea,")}
                      <br />
                      {t("or refine what’s already here.")}
                    </p>
                  </div>
                  <div className="suggestions">
                    <span>{t("A FEW PLACES TO START")}</span>
                    {[
                      "Создай шестерёнку с 20 зубьями",
                      "Change the mounting holes to 6 mm",
                      "Создай фигурку кота высотой 60 мм",
                    ].map((suggestion) => (
                      <button
                        key={suggestion}
                        onClick={() => {
                          setPrompt(t(suggestion));
                          chatInput.current?.focus();
                        }}
                      >
                        <Box size={14} />
                        {t(suggestion)}
                        <ArrowUpRight size={13} />
                      </button>
                    ))}
                  </div>
                </>
              ) : (
                chatMessages.map((m) => (
                  <motion.div
                    initial={{ opacity: 0, y: 5 }}
                    animate={{ opacity: 1, y: 0 }}
                    className={`message ${m.role}`}
                    key={m.id}
                  >
                    {m.role === "user" ? (
                      <div className="message-label">
                        {t("You")}{" "}
                        <time>
                          {new Date(m.createdAt).toLocaleTimeString(
                            getLocale(),
                            {
                              hour: "2-digit",
                              minute: "2-digit",
                            },
                          )}
                        </time>
                      </div>
                    ) : m.role === "event" ? (
                      <CircleCheck size={15} />
                    ) : m.role === "error" ? (
                      <AlertCircle size={15} />
                    ) : (
                      <div className="message-label">
                        <Sparkles size={13} />{" "}
                        {project.agent === "codex" ? "Codex" : "Claude"}
                        <time>
                          {new Date(m.createdAt).toLocaleTimeString(
                            getLocale(),
                            {
                              hour: "2-digit",
                              minute: "2-digit",
                            },
                          )}
                        </time>
                      </div>
                    )}
                    <MessageText
                      text={m.role === "event" ? systemText(m.text) : m.text}
                      error={m.role === "error"}
                    />
                    {m.role === "assistant" && (
                      <button
                        className="message-copy"
                        aria-label={t("Copy assistant message")}
                        onClick={() =>
                          void navigator.clipboard
                            .writeText(m.text)
                            .then(() => setNotice(t("Message copied")))
                            .catch((e) => setError(errorText(e)))
                        }
                      >
                        <Copy size={12} />
                      </button>
                    )}
                    {m.role === "error" &&
                      !/usage limit|rate limit|quota/i.test(m.text) &&
                      chatMessages.some(
                        (message) => message.role === "user",
                      ) && (
                        <button
                          onClick={() => {
                            const last = [...chatMessages]
                              .reverse()
                              .find((m) => m.role === "user");
                            if (last) setPrompt(last.text);
                          }}
                        >
                          {t("Edit and retry")}
                          <ArrowUpRight size={12} />
                        </button>
                      )}
                  </motion.div>
                ))
              )}
              {chatBusy && liveEvents.length > 0 && (
                <div className="live-feed" aria-live="polite">
                  {liveEvents.map((event, index) => (
                    <div key={index} className={`live-event ${event.kind}`}>
                      <span className="live-dot" />
                      <p>
                        {event.kind === "message"
                          ? event.text
                          : systemText(event.text)}
                      </p>
                    </div>
                  ))}
                </div>
              )}
              {chatBusy && (
                <div className="task-timeline">
                  <LoaderCircle size={16} className="spin" />
                  <div>
                    <strong>{systemText(useWorkspace.getState().stage)}</strong>
                    <p>
                      {systemText(
                        liveEvents.at(-1)?.text ??
                          t("Starting a local session…"),
                      )}
                    </p>
                  </div>
                  <button
                    onClick={() =>
                      void cancelTask(project.id).catch((e) =>
                        setError(errorText(e)),
                      )
                    }
                  >
                    <Square size={12} />
                    {t("Stop")}
                  </button>
                </div>
              )}
              <div ref={end} />
            </div>
            <div className="composer-wrap">
              {selected && (
                <div className="selection-context">
                  <Box size={12} />
                  {selectedLabel}
                  <button onClick={() => setSelected(null)}>
                    <X size={12} />
                  </button>
                </div>
              )}
              {attachments.length > 0 && (
                <div className="attachment-chips">
                  {attachments.map((a, i) => (
                    <span key={`${a.name}-${i}`}>
                      {a.kind === "image" && a.data && (
                        <img src={a.data} alt="" />
                      )}
                      {a.name}
                      <button
                        aria-label={t("Remove {{value0}}", { value0: a.name })}
                        onClick={() =>
                          setAttachments((old) => old.filter((_, j) => j !== i))
                        }
                      >
                        <X size={11} />
                      </button>
                    </span>
                  ))}
                </div>
              )}
              <div className="composer">
                <textarea
                  ref={chatInput}
                  aria-label={t("Message your CAD assistant")}
                  placeholder={t("Describe your next change…")}
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                      e.preventDefault();
                      void send();
                    }
                  }}
                />
                <div className="composer-actions">
                  <IconButton
                    label={t("Attach a file")}
                    disabled={editingBlocked}
                    onClick={() => fileInput.current?.click()}
                  >
                    <Paperclip size={16} />
                  </IconButton>
                  <span>{t("CAD context included")}</span>
                  <button
                    className="send-button"
                    aria-label={t("Send prompt")}
                    disabled={!prompt.trim() || editingBlocked}
                    onClick={() => void send()}
                  >
                    {busy ? (
                      <LoaderCircle size={16} className="spin" />
                    ) : (
                      <ArrowUp size={17} />
                    )}
                  </button>
                </div>
              </div>
              <div className="composer-hint">
                <span>{t("Ctrl ↵ to send")}</span>
              </div>
            </div>
          </aside>
        </div>
      )}
      <footer className="statusbar">
        <span className="status-dot" />
        <span>
          {busy
            ? chatBusy
              ? t("Agent working")
              : systemText(useWorkspace.getState().stage)
            : native
              ? t("Local workspace ready")
              : t("Browser workspace · desktop required for AI")}
        </span>
        {project && (
          <>
            <span className="status-separator" />
            <GitBranch size={11} />
            <span>{quantity("revisions", project.revisions.length)}</span>
          </>
        )}
        <span className="statusbar-spacer" />
        <span>v{appVersion}</span>
      </footer>
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
      <ActionReviewDialog pending={pending} onDecision={allow => void approve(allow)} />
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
        importNames={attachments.map(file => file.name)}
        open={modal === "new"}
        close={() => setModal(null)}
        onCreate={async (p) => {
          try {
            if (attachments.length) p = (await prepareProjectImport(p, attachments)).project;
            const saved = await saveProject(p);
            await openProject(saved);
            setModal(null);
            setAttachments([]);
            refresh();
            const step = native && attachments.find(f => /\.st(e)?p$/i.test(f.name));
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
          nativeCadAvailable={agentEnvironment.data?.some((item) => item.name === "OpenCascade kernel" && item.available) ?? false}
          nativeCadChecked={!!agentEnvironment.data}
          onApply={(program) => {
            const base = useWorkspace.getState().project;
            if (!base) return;
            const captured = captureApply(base, program, t("Изменён исходный код модели"));
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
              }, "modify_project", base.id,
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
                if (!path) { setNotice(t("Export cancelled")); return; }
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
          chatInput.current?.focus();
        }}
        onSettings={() => setModal("settings")}
      />
    </div>
  );
}
