import {
  t,
  useLocale,
  quantity,
  fixedNumber,
  setLocale,
  getLocale,
  systemText,
  errorText,
  rawError,
} from "../i18n";
import { CadFeatureEditor } from "../components/CadFeatureEditor";
import { applyTheme } from "../lib/theme";
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
  FolderOpen,
  History,
  LoaderCircle,
  PanelLeftClose,
  Paperclip,
  Plus,
  Search,
  Settings2,
  ShieldCheck,
  Sparkles,
  Square,
  Star,
  Pencil,
  Trash2,
  Undo2,
  X,
  SlidersHorizontal,
  Check,
  RefreshCw,
  Copy,
  Cpu,
  AlertCircle,
  GitBranch,
  LayoutGrid,
  Eye,
  EyeOff,
} from "lucide-react";
import {
  Button,
  IconButton,
  Modal,
  Select,
  Checkbox,
  NumberInput,
  TreeFolder,
} from "../components/ui";
import {
  defaults,
  type Project,
  type Parameters,
  type Agent,
  type ProjectFile,
} from "../types";
import {
  useWorkspace,
  currentParameters,
  newProject,
} from "../stores/workspace";
import {
  readProjectFile,
  interruptedSessions,
  acknowledgeRecovery,
  convertStep,
  permissionAudit,
  confirmationSettings,
  saveConfirmationSettings,
  needsConfirmation,
  type ConfirmationSettings,
  listProjects,
  saveProject,
  saveProjectThumbnail,
  deleteProject,
  health,
  plan,
  native,
  cancelTask,
  exportStep,
  applyProgram,
  chooseCadPython,
  saveMesh,
  requestNativePermission,
  resolveNativePermission,
} from "../lib/api";
import {
  buildModel,
  inspectModel,
  disposeModel,
  parameterSchema,
} from "../lib/model";
import { readFile, loadModel, exportMesh, download } from "../lib/files";
import { renderThumbnail } from "../lib/thumbnail";
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
const bodyLabel = (object: THREE.Object3D) =>
  typeof object.userData.formaLabel === "string"
    ? t(object.userData.formaLabel)
    : object.name;
const legacyMotionPrompt = (text: string) =>
  text.startsWith("Добавь анимацию движения к текущей сборке.");
const revisionPrompt = (text: string) =>
  legacyMotionPrompt(text) ? t("Настроено движение сборки") : text;
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
    setSelected,
  } = useWorkspace();
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
  const [tab, setTab] = usePersistentState<"files" | "history">(`${stateKey}.tab`, "files");
  const [search, setSearch] = usePersistentState("forma.ui.search", "");
  const [renameTarget, setRenameTarget] = useState<Project | null>(null);
  const [renameName, setRenameName] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<Project | null>(null);
  const [deleteName, setDeleteName] = useState("");
  const previewProjects = useRef<Project[]>([]);
  const previewRunning = useRef(false);
  const previewAttempted = useRef(new Set<string>());
  const [prompt, setPrompt] = usePersistentState(`${stateKey}.prompt`, "");
  const [pending, setPending] = useState<{
    title: string;
    description: string;
    detail: string;
    run: () => Promise<void>;
    id?: string;
  } | null>(null);
  const [attachments, setAttachments] = usePersistentState<ProjectFile[]>(`${stateKey}.attachments`, []);
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
  const [leftWidth, setLeftWidth] = usePersistentState("forma.ui.leftWidth", 238);
  const [rightWidth, setRightWidth] = usePersistentState("forma.ui.rightWidth", 350);
  const [compare, setCompare] = usePersistentState<string | null>(`${stateKey}.compare`, null);
  const [imported, setImported] = useState<THREE.Group | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const chatInput = useRef<HTMLTextAreaElement>(null);
  const end = useRef<HTMLDivElement>(null);
  const parameters = currentParameters(project);
  const revision = project?.revisions.find(
    (r) => r.id === project.currentRevision,
  );
  const [loadingModel, setLoadingModel] = useState(false);
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
    () => buildModel(JSON.parse(geometryKey) as Parameters),
    [geometryKey],
  );
  const object = imported ?? model;
  const selectedBody = selected ? object.getObjectByName(selected) : undefined;
  const selectedLabel = selectedBody ? bodyLabel(selectedBody) : selected;
  const stats = useMemo(() => inspectModel(object), [object]);
  const [before, setBefore] = useState<THREE.Group | null>(null);
  const [hiddenBodies, setHiddenBodies] = usePersistentState<string[]>(`${stateKey}.hiddenBodies`, []);
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
          return item.currentRevision && item.thumbnailRevision !== item.currentRevision && !previewAttempted.current.has(key);
        });
        if (!next || !next.currentRevision) break;
        previewAttempted.current.add(`${next.id}:${next.currentRevision}`);
        const revision = next.revisions.find((item) => item.id === next.currentRevision);
        const file = next.files.find((item) => item.name === (revision?.preview ?? revision?.source));
        let preview: THREE.Group | null = null;
        try {
          preview = file && /\.(stl|obj|glb|3mf)$/i.test(file.name)
            ? await loadModel(file, next.id)
            : buildModel(currentParameters(next));
          const thumbnail = renderThumbnail(preview);
          if (!thumbnail) continue;
          const saved = await saveProjectThumbnail(next.id, next.currentRevision, thumbnail);
          queryClient.setQueryData<Project[]>(["projects"], (current) =>
            current?.map((item) => item.id === saved.id ? saved : item),
          );
        } catch (cause) {
          console.warn("Project thumbnail could not be created", cause);
        } finally {
          if (preview) disposeModel(preview);
        }
      }
    })().finally(() => { previewRunning.current = false; });
  }, [projectId, projects.data, queryClient]);
  useEffect(
    () => () => {
      if (imported) disposeModel(imported);
    },
    [imported],
  );
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
    let active = true;
    setImported(null);
    const file = project?.files.find(
      (f) => f.name === (revision?.preview ?? revision?.source),
    );
    if (file) {
      setLoadingModel(true);
      loadModel(file, project?.id)
        .then((m) => {
          if (active) setImported(m);
          else disposeModel(m);
        })
        .catch((e) => {
          if (active) setError(errorText(e));
        })
        .finally(() => {
          if (active) setLoadingModel(false);
        });
    }
    return () => {
      active = false;
    };
  }, [project?.id, revision?.id, revision?.source]);
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
      const input =
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement;
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
      } else if (e.key === "z" && !input && project && !busy) {
        e.preventDefault();
        const parent = project.revisions.find((r) => r.id === revision?.parent);
        if (parent)
          ask(
            t("Restore previous revision"),
            t("Create a new revision from the previous model."),
            parent.prompt,
            () =>
              revise(
                parent.parameters,
                t("Restored: {{value0}}", { value0: parent.prompt }),
                parent.source,
                parent.preview,
                parent.program,
                parent.programBase,
              ),
          );
      }
    };
    window.addEventListener("keydown", f);
    return () => window.removeEventListener("keydown", f);
  });
  function refresh() {
    void queryClient.invalidateQueries({ queryKey: ["projects"] });
  }
  async function ask(
    title: string,
    description: string,
    detail: string,
    run: () => Promise<void>,
    action = "modify_project",
  ) {
    try {
      if (!needsConfirmation(await confirmationSettings(), action)) {
        await run();
        refresh();
        return;
      }
      const id =
        native && project
          ? await requestNativePermission(project.id, action, detail)
          : undefined;
      setPending({ title, description, detail, run, id });
    } catch (e) {
      setError(errorText(e));
    }
  }
  async function approve(allow: boolean) {
    const p = pending;
    if (!p) return;
    setPending(null);
    try {
      if (p.id) await resolveNativePermission(p.id, allow);
      if (allow) await p.run();
      else setNotice(t("Action denied. No model changes were made."));
    } catch (e) {
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
          const merged = [...p.files];
          for (const f of importedFiles) {
            if (merged.some((x) => x.name === f.name))
              throw new Error(
                t(
                  "A file named {{value0}} already exists. Rename it before importing.",
                  { value0: f.name },
                ),
              );
            merged.push(f);
          }
          const mesh = importedFiles.find((f) =>
            /\.(stl|obj|glb|3mf)$/i.test(f.name),
          );
          if (mesh) {
            const test = await loadModel(mesh);
            disposeModel(test);
            const importedRevision = {
              id: crypto.randomUUID(),
              parent: p.currentRevision,
              createdAt: stamp(),
              parameters: { ...defaults, kind: "blank" as const },
              prompt: t("Imported {{value0}}", { value0: mesh.name }),
              source: mesh.name,
            };
            await update({
              ...p,
              files: merged,
              updatedAt: stamp(),
              revisions: [...p.revisions, importedRevision],
              currentRevision: importedRevision.id,
            });
          } else {
            await update({ ...p, files: merged, updatedAt: stamp() });
            setAttachments((a) => [...a, ...importedFiles]);
            setNotice(t("Files added to project attachments."));
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
    if (!project || busy || !request.trim()) return;
    const text = request.trim();
    const p = project;
    await ask(
      t("Connect to your AI agent"),
      t(
        "The local CLI sends your request and current model source using your existing login.",
      ),
      t(
        "{{value0}} · {{value1}}\n\n{{value2}}\n\nThe agent writes CAD source for this request.",
        { value0: p.agent, value1: p.name, value2: text },
      ),
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
          );
          setAttachments([]);
          const latest = useWorkspace.getState().project;
          if (!latest) throw new Error(t("Project closed during the request"));
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
                if (!current)
                  throw new Error(t("Project closed during the request"));
                if (current.currentRevision !== p.currentRevision)
                  throw new Error(
                    t("Модель изменилась во время ответа. Повторите запрос."),
                  );
                const built = await applyProgram(
                  current,
                  result.program!,
                  text,
                );
                await update({
                  ...built,
                  messages: [
                    ...built.messages,
                    {
                      id: crypto.randomUUID(),
                      role: "assistant",
                      text: result.message,
                      createdAt: stamp(),
                    },
                  ],
                });
              } catch (error) {
                const current = useWorkspace.getState().project;
                if (current)
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
          );
        } catch (e) {
          const latest = useWorkspace.getState().project;
          if (latest)
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
              disabled={busy}
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
      {!project ? (
        <main className="dashboard">
          <div className="dashboard-nav">
            <div>
              <LayoutGrid size={17} />
              {t("All projects")}
            </div>
            <button onClick={() => setModal("settings")}>
              <Cpu size={17} />
              {t("AI agents")}
            </button>
            <button onClick={() => setModal("settings")}>
              <Settings2 size={17} />
              {t("Settings")}
            </button>
          </div>
          <div className="dashboard-content">
            <div className="eyebrow">{t("YOUR IDEAS, IN DIMENSIONS")}</div>
            <div className="dashboard-title">
              <div>
                <h1>{t("A place to make things.")}</h1>
                <p>{t("Give your next idea a little more shape.")}</p>
              </div>
              <Button onClick={() => fileInput.current?.click()}>
                <FolderOpen size={15} />
                {t("Import model")}
              </Button>
            </div>
            <div className="section-heading">
              <h2>{t("Projects")}</h2>
              <div className="search-input">
                <Search size={14} />
                <input
                  aria-label={t("Search projects")}
                  placeholder={t("Find a project…")}
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </div>
            </div>
            {projects.isLoading ? (
              <div className="empty-state">
                <LoaderCircle className="spin" />
                {t("Loading projects…")}
              </div>
            ) : projects.error ? (
              <div className="error-inline">{errorText(projects.error)}</div>
            ) : (
              <div className="project-grid">
                <button
                  className="new-project-card"
                  onClick={() => setModal("new")}
                >
                  <span>
                    <Plus size={24} />
                  </span>
                  <strong>{t("Create a project")}</strong>
                  <small>{t("From a thought to a tangible thing")}</small>
                </button>
                {[...(projects.data ?? [])]
                  .sort((a, b) => Number(b.pinned) - Number(a.pinned))
                  .filter((p) =>
                    [
                      p.name,
                      ...p.files.map((f) => f.name),
                      ...p.messages.map((m) => m.text),
                      ...p.revisions.map((r) => r.prompt),
                    ].some((text) =>
                      text.toLowerCase().includes(search.toLowerCase()),
                    ),
                  )
                  .map((p) => (
                    <motion.article
                      layout="position"
                      transition={{ layout: { duration: 0.42, ease: [0.22, 1, 0.36, 1] } }}
                      className={`project-card${p.pinned ? " is-pinned" : ""}`}
                      key={p.id}
                    >
                      <button
                        className="project-art"
                        onClick={() => void openProject(p)}
                      >
                        {p.thumbnail && p.thumbnailRevision === p.currentRevision ? (
                          <img src={p.thumbnail} alt={t("Preview of {{value0}}", { value0: p.name })} />
                        ) : (
                          <Box size={72} strokeWidth={0.7} />
                        )}
                        {p.pinned && <span className="project-pinned"><Star size={12} fill="currentColor" />{t("Pinned")}</span>}
                        <span>{currentParameters(p).kind.toUpperCase()}</span>
                      </button>
                      <div className="project-card-info">
                        <button onClick={() => void openProject(p)}>
                          <strong>{p.name}</strong>
                          <small>
                            {date(p.updatedAt)} <span>·</span>{" "}
                            {quantity("revisions", p.revisions.length)}
                          </small>
                        </button>
                        <div className="project-card-actions">
                          <IconButton
                            label={p.pinned ? t("Unpin project") : t("Pin project")}
                            aria-pressed={p.pinned}
                            active={p.pinned}
                            onClick={() => {
                              const current = queryClient.getQueryData<Project[]>(["projects"]) ?? [];
                              const latest = current.find((item) => item.id === p.id) ?? p;
                              const next = { ...latest, pinned: !latest.pinned };
                              queryClient.setQueryData<Project[]>(["projects"], current.map((item) => item.id === p.id ? next : item));
                              void saveProject(next).then(refresh).catch((cause) => {
                                queryClient.setQueryData(["projects"], current);
                                setError(errorText(cause));
                              });
                            }}
                          >
                            <Star size={15} fill={p.pinned ? "currentColor" : "none"} />
                          </IconButton>
                          <IconButton label={t("Rename project")} onClick={() => { setRenameTarget(p); setRenameName(p.name); }}>
                            <Pencil size={14} />
                          </IconButton>
                          <IconButton label={t("Delete project")} onClick={() => { setDeleteTarget(p); setDeleteName(""); }}>
                            <Trash2 size={14} />
                          </IconButton>
                        </div>
                      </div>
                    </motion.article>
                  ))}
              </div>
            )}
            <div className="dashboard-footer">
              <span>FORMA / {appVersion}</span>
            </div>
          </div>
        </main>
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
                    disabled={busy}
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
                          disabled={busy}
                          onClick={() => setModal("parameters")}
                        >
                          <SlidersHorizontal size={13} />
                        </IconButton>
                      </div>
                      <TreeFolder folderId="assembly" title={t("Assembly")} count={stats.bodies}>
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
                                  if (next.has(`${i}:${o.name}`)) next.delete(`${i}:${o.name}`);
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
                          disabled={busy}
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
                        folderId="attachments" title={t("attachments")}
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
                        folderId="revisions" title={t("Revisions")}
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
                        folderId="exports" title={t("exports")}
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
                              disabled={busy}
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
                <div className="inspector">
                  <div className="inspector-title">
                    <Box size={13} />
                    <span>{selectedLabel || t("Model properties")}</span>
                    <button onClick={() => setModal("parameters")}>
                      <SlidersHorizontal size={13} />
                    </button>
                  </div>
                  <dl>
                    <div>
                      <dt>{t("Dimensions")}</dt>
                      <dd>
                        {stats.size
                          .map((n) =>
                            fixedNumber(
                              n /
                                (project.units === "inch"
                                  ? 25.4
                                  : project.units === "cm"
                                    ? 10
                                    : 1),
                              1,
                            ),
                          )
                          .join(" × ")}{" "}
                        <span>{t(project.units)}</span>
                      </dd>
                    </div>
                    <div>
                      <dt>{t("Bodies")}</dt>
                      <dd>{stats.bodies}</dd>
                    </div>
                    <div>
                      <dt>{t("Triangles")}</dt>
                      <dd>{stats.triangles.toLocaleString(getLocale())}</dd>
                    </div>
                    <div>
                      <dt>{t("Surface area¹")}</dt>
                      <dd>
                        {fixedNumber(stats.area / 100, 1)} {t("cm²")}
                      </dd>
                    </div>
                  </dl>
                  <small>
                    {t("¹ Mesh estimate; overlapping faces included.")}
                  </small>
                  {parameters.thickness < 1 && (
                    <div className="thin-warning">
                      <AlertCircle size={13} />
                      {t("Thin walls may be difficult to print.")}
                    </div>
                  )}
                </div>
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
            <div className={`viewer-area ${before ? "comparison" : ""}`}>
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
                    onScreenshot={(data) => void captureScreenshot(data)}
                  />
                </Suspense>
              </ViewerBoundary>
              {loadingModel && (
                <div className="model-loading">
                  <LoaderCircle className="spin" />
                  {t("Reading geometry…")}
                </div>
              )}
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
                        liveEvents.at(-1)?.text ?? t("Starting a local session…"),
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
                    disabled={busy}
                    onClick={() => fileInput.current?.click()}
                  >
                    <Paperclip size={16} />
                  </IconButton>
                  <span>{t("CAD context included")}</span>
                  <button
                    className="send-button"
                    aria-label={t("Send prompt")}
                    disabled={!prompt.trim() || busy}
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
            ? chatBusy ? t("Agent working") : systemText(useWorkspace.getState().stage)
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
      <Modal
        open={!!pending}
        onClose={() => void approve(false)}
        title={systemText(pending?.title ?? t("Review action"))}
        description={systemText(pending?.description ?? "")}
      >
        <div className="permission-detail">
          <ShieldCheck size={20} />
          <pre>{pending?.detail}</pre>
        </div>

        <div className="modal-actions">
          <Button onClick={() => void approve(false)}>{t("Deny")}</Button>
          <Button className="primary" onClick={() => void approve(true)}>
            <Check size={15} />
            {t("Allow once")}
          </Button>
        </div>
      </Modal>
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
        description={t("The model and its history will keep their current files.")}
      >
        <form onSubmit={(event) => {
          event.preventDefault();
          if (!renameTarget || !renameName.trim()) return;
          const latest = (queryClient.getQueryData<Project[]>(["projects"]) ?? []).find((item) => item.id === renameTarget.id) ?? renameTarget;
          void saveProject({ ...latest, name: renameName.trim(), updatedAt: stamp() })
            .then(() => { setRenameTarget(null); refresh(); setNotice(t("Project renamed")); })
            .catch((cause) => setError(errorText(cause)));
        }}>
          <label>{t("Project name")}
            <input autoFocus value={renameName} maxLength={80} onChange={(event) => setRenameName(event.target.value)} />
          </label>
          <div className="modal-actions">
            <Button type="button" onClick={() => setRenameTarget(null)}>{t("Cancel")}</Button>
            <Button className="primary" type="submit" disabled={!renameName.trim()}>{t("Save name")}</Button>
          </div>
        </form>
      </Modal>
      <Modal
        open={!!deleteTarget}
        onClose={() => setDeleteTarget(null)}
        title={t("Delete project")}
        description={t("This permanently deletes the project, its revisions, attachments and stored files.")}
      >
        <form onSubmit={(event) => {
          event.preventDefault();
          if (!deleteTarget || deleteName !== deleteTarget.name) return;
          void deleteProject(deleteTarget.id)
            .then(() => {
              queryClient.setQueryData<Project[]>(["projects"], (current) => current?.filter((item) => item.id !== deleteTarget.id));
              setDeleteTarget(null);
              refresh();
              setNotice(t("Project deleted"));
            })
            .catch((cause) => setError(errorText(cause)));
        }}>
          <label>
            {t("delete.confirmBefore")} <strong className="delete-project-name">{deleteTarget?.name}</strong>{t("delete.confirmAfter")}
            <input autoFocus autoComplete="off" value={deleteName} onChange={(event) => setDeleteName(event.target.value)} />
          </label>
          <div className="modal-actions">
            <Button type="button" onClick={() => setDeleteTarget(null)}>{t("Cancel")}</Button>
            <Button className="danger" type="submit" disabled={deleteName !== deleteTarget?.name}>
              <Trash2 size={14} />{t("Delete permanently")}
            </Button>
          </div>
        </form>
      </Modal>
      <NewProjectDialog
        open={modal === "new"}
        close={() => setModal(null)}
        onCreate={async (p) => {
          try {
            if (attachments.length) {
              p.files = attachments;
              const mesh = attachments.find((f) =>
                /\.(stl|obj|glb|3mf)$/i.test(f.name),
              );
              if (mesh) {
                const check = await loadModel(mesh);
                disposeModel(check);
                const r = {
                  id: crypto.randomUUID(),
                  parent: null,
                  createdAt: stamp(),
                  parameters: { ...defaults, kind: "blank" as const },
                  prompt: t("Imported {{value0}}", { value0: mesh.name }),
                  source: mesh.name,
                };
                p.revisions = [r];
                p.currentRevision = r.id;
              }
            }
            await openProject(await saveProject(p));
            setModal(null);
            refresh();
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
          disabled={busy}
          onApply={(program) => {
            setModal(null);
            void ask(
              t("Построить модель"),
              t("Проверить геометрию и сохранить ревизию."),
              program,
              async () => {
                const current = useWorkspace.getState().project;
                if (!current) return;
                setBusy(true, t("Построение геометрии"));
                try {
                  useWorkspace
                    .getState()
                    .setProject(
                      await applyProgram(
                        current,
                        program,
                        t("Изменён исходный код модели"),
                      ),
                    );
                } finally {
                  setBusy(false);
                }
              },
            );
          }}
        />
      ) : (
        <ParametersDialog
          open={modal === "parameters"}
          close={() => setModal(null)}
          parameters={parameters}
          disabled={busy}
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
        onExport={async (format) => {
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
                const path = await exportStep(project.id, parameters);
                setNotice(t("STEP exported to {{value0}}", { value0: path }));
              } else {
                const blob = await exportMesh(object, format);
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
      <Modal
        open={modal === "palette"}
        onClose={() => setModal(null)}
        title={t("Command palette")}
        description={t("Find a project or jump to an action.")}
      >
        <div className="palette-search">
          <Search size={17} />
          <input
            autoFocus
            placeholder={t("Search commands and projects…")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <kbd>ESC</kbd>
        </div>
        <div className="palette-list">
          {[
            {
              label: t("New project"),
              shortcut: "Ctrl N",
              run: () => setModal("new"),
            },
            {
              label: t("Import model"),
              shortcut: "Ctrl O",
              run: () => {
                setModal(null);
                fileInput.current?.click();
              },
            },
            {
              label: t("Export model"),
              shortcut: "Ctrl Shift E",
              run: () => setModal("export"),
            },
            {
              label: t("Edit model parameters"),
              shortcut: "",
              run: () => setModal("parameters"),
            },
            {
              label: t("Ask AI"),
              shortcut: "Ctrl Enter",
              run: () => {
                setModal(null);
                chatInput.current?.focus();
              },
            },
            {
              label: t("Open settings"),
              shortcut: "",
              run: () => setModal("settings"),
            },
          ]
            .filter((c) => c.label.toLowerCase().includes(search.toLowerCase()))
            .map((c) => (
              <button key={c.label} onClick={c.run}>
                <Command size={15} />
                {c.label}
                <kbd>{c.shortcut}</kbd>
              </button>
            ))}
          {projects.data
            ?.filter((p) =>
              [
                p.name,
                ...p.files.map((f) => f.name),
                ...p.messages.map((m) => m.text),
                ...p.revisions.map((r) => r.prompt),
              ].some((text) =>
                text.toLowerCase().includes(search.toLowerCase()),
              ),
            )
            .map((p) => (
              <button
                key={p.id}
                onClick={() => {
                  void openProject(p);
                  setModal(null);
                }}
              >
                <Folder size={15} />
                {p.name}
                <ArrowUpRight size={13} />
              </button>
            ))}
        </div>
      </Modal>
    </div>
  );
}
function NewProjectDialog({
  open,
  close,
  onCreate,
}: {
  open: boolean;
  close: () => void;
  onCreate: (p: Project) => Promise<void>;
}) {
  const [name, setName] = useState(t("Untitled part"));
  const [units, setUnits] = useState<Project["units"]>("mm");
  const [agent, setAgent] = useState<Agent>("codex");
  const [saving, setSaving] = useState(false);
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Make room for an idea")}
      description={t("Start with an empty workspace.")}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!name.trim() || saving) return;
          setSaving(true);
          void onCreate(newProject(name, "blank", units, agent)).finally(() =>
            setSaving(false),
          );
        }}
      >
        <label>
          {t("Project name")}
          <input
            required
            maxLength={80}
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <div className="form-row">
          <label>
            {t("Display units")}
            <Select
              value={units}
              onChange={(e) => setUnits(e.target.value as Project["units"])}
            >
              <option value="mm">{t("Millimeters")}</option>
              <option value="cm">{t("Centimeters")}</option>
              <option value="inch">{t("Inches")}</option>
            </Select>
          </label>
          <label>
            {t("AI agent")}
            <Select
              value={agent}
              onChange={(e) => setAgent(e.target.value as Agent)}
            >
              <option value="codex">OpenAI Codex</option>
              <option value="claude">Claude Code</option>
              <option value="custom">{t("Custom CLI")}</option>
            </Select>
          </label>
        </div>
        <div className="modal-actions">
          <Button type="button" onClick={close}>
            {t("Cancel")}
          </Button>
          <Button
            className="primary"
            type="submit"
            disabled={saving || !name.trim()}
          >
            {saving ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <Plus size={15} />
            )}
            {t("Create project")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function SettingsDialog({
  open,
  close,
  project,
  onAgent,
}: {
  open: boolean;
  close: () => void;
  project: Project | null;
  onAgent: (a: Agent) => Promise<void>;
}) {
  const system = useQuery({
    queryKey: ["health"],
    queryFn: health,
    enabled: open,
    staleTime: 30000,
  });
  const [page, setPage] = usePersistentState("forma.ui.settingsPage", "AI agents");
  const [configuring, setConfiguring] = useState(false);
  const [configurationError, setConfigurationError] = useState("");
  const audit = useQuery({
    queryKey: ["permission-audit"],
    queryFn: permissionAudit,
    enabled: open && page === "Permissions",
    staleTime: 0,
  });
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Settings")}
      description={t("Application preferences")}
      wide
    >
      <div className="settings-layout">
        <nav>
          {[
            "AI agents",
            "CAD environment",
            "Permissions",
            "Appearance",
            "Privacy",
          ].map((s) => (
            <button
              className={page === s ? "active" : ""}
              onClick={() => setPage(s)}
              key={s}
            >
              {t(s)}
            </button>
          ))}
        </nav>
        <section>
          {page === "AI agents" || page === "CAD environment" ? (
            <>
              <div className="settings-section-title">
                <h3>{t(page)}</h3>
                <IconButton
                  label={t("Refresh environment")}
                  onClick={() => void system.refetch()}
                >
                  <RefreshCw size={14} />
                </IconButton>
              </div>
              {system.isFetching && <LoaderCircle size={17} className="spin" />}
              {system.error && (
                <p className="error-inline">{errorText(system.error)}</p>
              )}
              {system.data
                ?.filter((h) =>
                  page === "AI agents"
                    ? /codex|claude|desktop runtime/i.test(h.name)
                    : !/codex|claude|desktop runtime/i.test(h.name),
                )
                .map((h) => (
                  <div className="health-row" key={h.name}>
                    <span
                      className={`health-icon ${h.available ? "connected" : ""}`}
                    >
                      {h.available ? <Check size={15} /> : <Cpu size={15} />}
                    </span>
                    <div>
                      <strong>{t(h.name)}</strong>
                      <small>{systemText(h.detail)}</small>
                    </div>
                    <span>
                      {h.available ? t("Detected") : t("Unavailable")}
                    </span>
                  </div>
                ))}
              {native && page === "CAD environment" && (
                <>
                  <Button
                    className="cad-python-action"
                    disabled={configuring}
                    onClick={async () => {
                      setConfiguring(true);
                      setConfigurationError("");
                      try {
                        await chooseCadPython();
                        await system.refetch();
                      } catch (error) {
                        setConfigurationError(errorText(error));
                      } finally {
                        setConfiguring(false);
                      }
                    }}
                  >
                    {configuring
                      ? t("Проверка CadQuery…")
                      : t("Выбрать Python с CadQuery")}
                  </Button>
                  {configurationError && (
                    <p className="error-inline">{configurationError}</p>
                  )}
                </>
              )}
              {project && page === "AI agents" && (
                <label>
                  {t("Project agent")}
                  <Select
                    value={project.agent}
                    onChange={(e) => void onAgent(e.target.value as Agent)}
                  >
                    <option value="codex">OpenAI Codex</option>
                    <option value="claude">Claude Code</option>
                    <option value="custom">
                      {t("Custom CLI (not configured)")}
                    </option>
                  </Select>
                </label>
              )}
              <p className="field-hint">
                {page === "AI agents"
                  ? t("Uses your existing CLI login.")
                  : t("Python and CadQuery power STEP import and export.")}
              </p>
            </>
          ) : page === "Permissions" ? (
            <>
              <ConfirmationControls />
              <div className="settings-section-title">
                <h3>{t("Recent decisions")}</h3>
                <IconButton
                  label={t("Refresh audit log")}
                  onClick={() => void audit.refetch()}
                >
                  <RefreshCw size={14} />
                </IconButton>
              </div>
              {audit.isFetching && <LoaderCircle className="spin" size={16} />}
              {audit.error && (
                <p className="error-inline">{errorText(audit.error)}</p>
              )}
              {!native && (
                <p>
                  {t("The permission journal is available in the desktop app.")}
                </p>
              )}
              {native && audit.data?.length === 0 && (
                <p>{t("No permission requests recorded.")}</p>
              )}
              <div style={{ maxHeight: 320, overflowY: "auto" }}>
                {audit.data?.map((entry) => (
                  <div className="health-row" key={entry.id}>
                    <ShieldCheck size={16} />
                    <div>
                      <strong>
                        {t(entry.action)} · {t(entry.decision)}
                      </strong>
                      <small>{systemText(entry.detail)}</small>
                      <small>
                        {new Date(entry.createdAt).toLocaleString(getLocale())}{" "}
                        · {entry.projectId.slice(0, 8)}
                      </small>
                    </div>
                  </div>
                ))}
              </div>
              <p className="field-hint">
                {t(
                  "Latest 100 recorded decisions. An allowed decision does not mean the operation completed. Grants expire and cannot be reused.",
                )}
              </p>
            </>
          ) : page === "Appearance" ? (
            <>
              <label>
                {t("Language")}
                <Select
                  aria-label={t("Language")}
                  value={getLocale()}
                  onChange={(event) =>
                    setLocale(event.target.value as "en" | "ru")
                  }
                >
                  <option value="ru">Русский</option>
                  <option value="en">English</option>
                </Select>
              </label>
              <h3>{t("Appearance")}</h3>
              <label>
                {t("Theme")}
                <Select
                  defaultValue={localStorage.getItem("forma.theme") ?? "dark"}
                  onChange={(e) => {
                    applyTheme(e.target.value);
                  }}
                >
                  <option value="dark">{t("Dark")}</option>
                  <option value="light">{t("Light")}</option>
                </Select>
              </label>
              <p>
                {t(
                  "The 3D viewport and controls follow the selected theme.",
                )}
              </p>
            </>
          ) : (
            <>
              <ShieldCheck size={28} />
              <h3>{t("Your work stays yours.")}</h3>
              <p>
                {t(
                  "No developer backend and no analytics. Projects are saved locally. AI prompts go to your selected agent’s provider only when you approve a connection.",
                )}
              </p>
              <p>
                {t(
                  "Browser projects remain in browser storage. Clear site data only after exporting your work.",
                )}
              </p>
            </>
          )}
        </section>
      </div>
    </Modal>
  );
}
function ParametersDialog({
  open,
  close,
  parameters,
  onApply,
  disabled,
}: {
  open: boolean;
  close: () => void;
  parameters: Parameters;
  onApply: (p: Parameters) => void;
  disabled: boolean;
}) {
  const draftKey = useWorkspace(s => `forma.ui.project.${s.project?.id ?? "home"}.parameters.${s.project?.currentRevision ?? "new"}`);
  const [draft, setDraft] = usePersistentState(draftKey, parameters);
  const [error, setError] = useState("");
  useEffect(() => {
    if (open) {
      setError("");
    }
  }, [open, parameters]);
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Model parameters")}
      description={t(
        "Dimensions are stored in millimeters. Changes create a new revision.",
      )}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const result = parameterSchema.safeParse(draft);
          if (!result.success) {
            setError(t("Invalid parameter value."));
            return;
          }
          onApply(result.data);
        }}
      >
        <div className="parameter-grid">
          {(
            [
              "width",
              "depth",
              "height",
              "thickness",
              "holeDiameter",
              "holes",
            ] as const
          ).map((key) => (
            <label key={key}>
              {
                {
                  width: t("Width"),
                  depth: t("Depth"),
                  height: t("Height"),
                  thickness: t("Wall thickness"),
                  holeDiameter: t("Hole diameter"),
                  holes: t("Number of holes"),
                }[key]
              }
              <div className="number-field">
                <NumberInput
                  disabled={disabled}
                  aria-label={t(key)}
                  type="number"
                  step={key === "holes" ? 1 : 0.1}
                  value={draft[key]}
                  onChange={(e) =>
                    setDraft({ ...draft, [key]: e.target.valueAsNumber })
                  }
                />
                <span>{key === "holes" ? "" : t("mm")}</span>
              </div>
            </label>
          ))}
        </div>
        {disabled && (
          <p className="field-hint">
            {t(
              "Imported meshes are inspected and exported as geometry. Parametric edits apply to native templates.",
            )}
          </p>
        )}
        {error && (
          <p className="error-inline" role="alert">
            {errorText(error)}
          </p>
        )}
        <div className="modal-actions">
          <Button type="button" onClick={close}>
            {t("Cancel")}
          </Button>
          <Button disabled={disabled} className="primary" type="submit">
            {t("Review changes")}
            <ArrowUpRight size={14} />
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function ExportDialog({
  open,
  close,
  onExport,
  hasModel,
}: {
  open: boolean;
  close: () => void;
  onExport: (s: string) => Promise<void>;
  hasModel: boolean;
}) {
  const [format, setFormat] = usePersistentState("forma.ui.exportFormat", "stl");
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Ready for the next step")}
      description={t("Choose an export format.")}
    >
      <label>
        {t("File format")}
        <Select value={format} onChange={(e) => setFormat(e.target.value)}>
          <option value="stl">{t("STL — 3D printing mesh")}</option>
          <option value="3mf">{t("3MF — 3D printing assembly")}</option>
          <option value="glb">{t("GLB — portable 3D model")}</option>
          <option value="obj">{t("OBJ — polygon mesh")}</option>
          <option value="step" disabled={!native}>
            {t("STEP — solid CAD (desktop + CadQuery)")}
          </option>
        </Select>
      </label>
      <div className="export-spec">
        <span>
          {t("Units")}
          <strong>{t("Millimeters")}</strong>
        </span>
        <span>
          {t("Geometry")}
          <strong>
            {format === "step" ? t("Exact solid") : t("Current mesh")}
          </strong>
        </span>
      </div>
      <p className="field-hint">
        {t(
          "STL and OBJ are unitless formats; exported coordinates are in millimeters. Export does not alter your project.",
        )}
      </p>
      <div className="modal-actions">
        <Button onClick={close}>{t("Cancel")}</Button>
        <Button
          className="primary"
          disabled={!hasModel}
          onClick={() => void onExport(format)}
        >
          <Download size={15} />
          {t("Export")} {format.toUpperCase()}
        </Button>
      </div>
    </Modal>
  );
}

function ConfirmationControls() {
  const query = useQuery({
    queryKey: ["confirmations"],
    queryFn: confirmationSettings,
  });
  const client = useQueryClient();
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const value = query.data;
  async function save(next: ConfirmationSettings) {
    setSaving(true);
    try {
      await saveConfirmationSettings(next);
      client.setQueryData(["confirmations"], next);
      setError("");
    } catch (error) {
      setError(errorText(error));
    } finally {
      setSaving(false);
    }
  }
  if (!value)
    return (
      <p>{query.error ? errorText(query.error) : t("Loading settings…")}</p>
    );
  return (
    <fieldset disabled={saving} className="confirmation-controls">
      <h3>{t("Confirmations")}</h3>
      <Select
        aria-label={t("Confirmation mode")}
        value={value.mode}
        onChange={(e) =>
          void save({
            mode: e.target.value as ConfirmationSettings["mode"],
            overrides: {},
          })
        }
      >
        <option value="all">{t("Confirm everything")}</option>
        <option value="cli">{t("Only CLI and dependencies")}</option>
        <option value="none">{t("No confirmations")}</option>
      </Select>
      <p className="field-hint">{t("Customize individual actions below.")}</p>
      {Object.entries({
        run_agent: "Connect to CLI agent",
        modify_project: "Edit models and project files",
        convert_file: "Convert CAD files",
        export_file: "Export files",
        install_dependency: "Install dependencies",
      }).map(([action, label]) => (
        <Checkbox
          key={action}
          checked={needsConfirmation(value, action)}
          onChange={(checked) =>
            void save({
              ...value,
              overrides: { ...value.overrides, [action]: checked },
            })
          }
        >
          {t(label)}
        </Checkbox>
      ))}
      {error && <p className="error-inline">{errorText(error)}</p>}
    </fieldset>
  );
}

function MessageText({ text, error }: { text: string; error: boolean }) {
  const quota = error && /usage limit|rate limit|quota/i.test(text);
  const retry = text.match(/try again at ([^.\n]+)/i)?.[1];
  if (quota)
    return (
      <div className="quota-message">
        <strong>{t("Лимит AI исчерпан")}</strong>
        <p>
          {retry
            ? t("Codex предлагает повторить запрос в {{value0}}.", {
                value0: retry,
              })
            : t("Провайдер временно не принимает новые запросы.")}
        </p>
        <p>{t("Модель и история сохранены.")}</p>
        <a
          href="https://chatgpt.com/codex/settings/usage"
          target="_blank"
          rel="noreferrer"
        >
          {t("Лимиты и кредиты ↗")}
        </a>
        <details>
          <summary>{t("Сообщение CLI")}</summary>
          <p>{text}</p>
        </details>
      </div>
    );
  if (error && text.trimStart().startsWith("{"))
    return (
      <div>
        <p>{errorText(text)}</p>
        <details>
          <summary>{t("Technical details")}</summary>
          <pre>{text}</pre>
        </details>
      </div>
    );
  return <p>{error ? errorText(text) : text}</p>;
}

function ProgramDialog({
  open,
  close,
  source,
  disabled,
  onApply,
}: {
  open: boolean;
  close: () => void;
  source: string;
  disabled: boolean;
  onApply: (source: string) => void;
}) {
  const draftKey = useWorkspace(s => `forma.ui.project.${s.project?.id ?? "home"}.source.${s.project?.currentRevision ?? "new"}`);
  const [draft, setDraft] = usePersistentState(draftKey, source);
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Исходный код модели")}
      description={
        draft.trimStart().startsWith("{")
          ? t("CAD features · миллиметры · параметры и зависимости")
          : t("CadQuery fallback · миллиметры · результат в переменной result")
      }
      wide
    >
      <CadFeatureEditor
        source={draft}
        disabled={disabled}
        onChange={setDraft}
      />
      <textarea
        className="program-editor"
        aria-label={t("CAD source")}
        value={draft}
        spellCheck={false}
        onChange={(event) => setDraft(event.target.value)}
        disabled={disabled}
      />
      {!source && (
        <p className="field-hint">
          {t("Опишите деталь в чате или напишите программу построения.")}
        </p>
      )}
      <div className="modal-actions">
        <Button onClick={close}>{t("Закрыть")}</Button>
        <Button
          className="primary"
          disabled={disabled || !draft.trim() || !native}
          onClick={() => onApply(draft)}
        >
          {t("Построить")}
        </Button>
      </div>
    </Modal>
  );
}
