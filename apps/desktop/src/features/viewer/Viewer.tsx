import { AdaptiveGrid } from "./grid/AdaptiveGrid";
import { useSectionView } from "./sections/useSectionView";
import { SectionControls } from "./sections/SectionControls";
import { SectionScene } from "./sections/SectionScene";
import { Scissors } from "lucide-react";
import { t } from "../../i18n";
import { useTheme } from "../../lib/theme";
import { usePersistentState } from "../../lib/persistence";
import { Canvas } from "@react-three/fiber";
import {
  OrbitControls,
  GizmoHelper,
  GizmoViewport,
  Bounds,
} from "@react-three/drei";
import { useCallback, useState, useRef, useMemo, useEffect } from "react";
import { ViewportSidebar } from "../viewport-sidebar/ViewportSidebar";
import { useNavigationPreferences } from "../settings/workspacePreferences";
import { OrbitSelection } from "./navigation/OrbitSelection";
import * as THREE from "three";
import {
  Box,
  Scan,
  Grid3X3,
  Maximize,
  Layers,
  MousePointer2,
  Ruler,
  Eye,
  Camera,
  RotateCcw,
  Play,
  Pause,
  Square,
  MoveRight,
} from "lucide-react";
import { IconButton, Select } from "../../components/ui";
import { useWorkspace } from "../../stores/workspace";
import { SmoothZoom, Playback, hasMotion } from "./Playback";
import { measurement, type MeasureKind } from "../../lib/measurement";
import { Model } from "./Model";
import { CameraControl, type OrbitHandle } from "./CameraControl";
import { Capture } from "./Capture";
import { PluginToolbarMenu } from "../plugins/PluginToolbarMenu";
import type { RenderMode } from "./renderMode";
import { faceTriangleCounts } from "./faceSelection";
import { cadEdges } from "./edgeSelection";
export type { RenderMode } from "./renderMode";
export default function Viewer({
  object,
  onScreenshot,
  comparison,
  interactive = true,
}: {
  object: THREE.Group;
  onScreenshot: (data: string) => void;
  comparison?: boolean;
  interactive?: boolean;
}) {
  const projectId = useWorkspace((s) => s.project?.id ?? "empty");
  const viewKey = `forma.ui.project.${projectId}.view.${comparison ? "comparison" : "main"}`;
  const navigation = useNavigationPreferences();
  const [toolbarOpen, setToolbarOpen] = usePersistentState(
    `${viewKey}.toolbarOpen`,
    true,
  );
  const [sidebarOpen, setSidebarOpen] = usePersistentState(
    `${viewKey}.sidebarOpen`,
    false,
  );
  const section = useSectionView(object, viewKey, !!comparison, !interactive);
  const [playing, setPlaying] = usePersistentState(`${viewKey}.playing`, false);
  const light = useTheme() === "light";
  const [motionStarted, setMotionStarted] = usePersistentState(
    `${viewKey}.motionStarted`,
    false,
  );
  const [motionReset, setMotionReset] = useState(0);
  const motion = hasMotion(object);
  const [reset, setReset] = useState(0);
  const [mode, setMode] = usePersistentState<RenderMode>(
    `${viewKey}.mode`,
    "edges",
  );
  const [grid, setGrid] = usePersistentState(`${viewKey}.grid`, true);
  const [preset, setPreset] = usePersistentState(`${viewKey}.preset`, "iso");
  const [ortho, setOrtho] = usePersistentState(`${viewKey}.ortho`, false);
  const [fit, setFit] = useState(0);
  const [measure, setMeasure] = useState(false);
  const [selectionMode, setSelectionMode] = useState<"body" | "face" | "edge">(
    "body",
  );
  const [measureKind, setMeasureKind] = useState<MeasureKind>("distance");
  const [points, setPoints] = useState<THREE.Vector3[]>([]);
  const [capture, setCapture] = useState<(() => string) | null>(null);
  const controlsRef = useRef<OrbitHandle | null>(null);
  const appliedPreset = useRef(preset);
  const selected = useWorkspace((s) => s.selected);
  const selectedFace = useWorkspace((s) => s.selectedFace);
  const selectedEdge = useWorkspace((s) => s.selectedEdge);
  const revisionId = useWorkspace((s) => s.project?.currentRevision ?? null);
  const hasCadFaces = useMemo(() => {
    if (!object.userData.formaNativeCadPreview) return false;
    let available = false;
    object.traverse((node) => {
      if (node instanceof THREE.Mesh && faceTriangleCounts(node))
        available = true;
    });
    return available;
  }, [object]);
  const hasCadEdges = useMemo(() => {
    if (!object.userData.formaNativeCadPreview) return false;
    let available = false;
    object.traverse((node) => {
      if (
        node instanceof THREE.Mesh &&
        cadEdges(node)?.some((edge) => edge.points.length > 1)
      )
        available = true;
    });
    return available;
  }, [object]);
  const handleCapture = useCallback(
    (fn: () => string) => setCapture(() => fn),
    [],
  );
  useEffect(() => {
    if (comparison) return;
    const keydown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (
        !target.closest('[data-editor="viewport3d"]') ||
        target.closest(
          "input,textarea,select,[contenteditable=true],[role=combobox]",
        ) ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey
      )
        return;
      if (event.code === "KeyT") {
        event.preventDefault();
        setToolbarOpen((value) => !value);
      }
      if (event.code === "KeyN") {
        event.preventDefault();
        setSidebarOpen((value) => !value);
      }
      const code = navigation.emulateNumpad
        ? event.code.replace("Digit", "Numpad")
        : event.code;
      const views: Record<string, string> = {
        Numpad1: "front",
        Numpad3: "side",
        Numpad7: "top",
      };
      if (views[code]) {
        event.preventDefault();
        setPreset(views[code]);
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [
    comparison,
    navigation.emulateNumpad,
    setToolbarOpen,
    setSidebarOpen,
    setPreset,
  ]);
  return (
    <div
      className={`viewport ${toolbarOpen ? "" : "workspace-viewer-tools-hidden"}`}
    >
      <div className="viewport-heading">
        <span>
          <Box size={13} />{" "}
          {comparison ? t("Revision comparison") : t("Model workspace")}
        </span>
        <span className="quiet">
          {ortho ? t("Orthographic") : t("Perspective")}{" "}
          <span className="tiny-dot" /> {t("mm")}
        </span>
        {!comparison && (
          <div className="workspace-viewport-panel-toggles">
            <button
              aria-pressed={toolbarOpen}
              aria-label={t("Toggle viewport toolbar (T)")}
              onClick={() => setToolbarOpen(!toolbarOpen)}
            >
              {t("Tools")}
            </button>
            <button
              aria-pressed={sidebarOpen}
              aria-label={t("Toggle viewport sidebar (N)")}
              onClick={() => setSidebarOpen(!sidebarOpen)}
            >
              {t("Item")}
            </button>
          </div>
        )}
      </div>
      {sidebarOpen && !comparison && (
        <ViewportSidebar
          object={object}
          selected={selected}
          onFit={() => setFit((value) => value + 1)}
          onReset={() => {
            setPreset("iso");
            setReset((value) => value + 1);
          }}
        />
      )}
      <div className="viewport-toolbars">
        <div className="view-toolbar">
          <IconButton
            label={t("Select body")}
            disabled={!interactive}
            active={!measure && selectionMode === "body"}
            onClick={() => {
              setMeasure(false);
              setSelectionMode("body");
            }}
          >
            <MousePointer2 size={17} />
          </IconButton>
          <IconButton
            label={
              hasCadFaces && !comparison
                ? t("Select CAD face")
                : t("CAD face selection requires a native model preview")
            }
            active={!measure && selectionMode === "face"}
            disabled={!interactive || !hasCadFaces || comparison}
            onClick={() => {
              setMeasure(false);
              setSelectionMode("face");
            }}
          >
            <Box size={17} />
          </IconButton>
          <IconButton
            label={t("Measure distance between two surface points")}
            disabled={!interactive}
            active={measure}
            onClick={() => {
              setMeasure(!measure);
              setPoints([]);
            }}
          >
            <Ruler size={17} />
          </IconButton>
          <IconButton
            label={
              hasCadEdges && !comparison
                ? t("Select CAD edge")
                : t("CAD edge selection requires a native model preview")
            }
            active={!measure && selectionMode === "edge"}
            disabled={!interactive || !hasCadEdges || comparison}
            onClick={() => {
              setMeasure(false);
              setSelectionMode("edge");
            }}
          >
            <MoveRight size={17} />
          </IconButton>
          <i />
          <IconButton
            label={
              section.available
                ? t("Section view")
                : t("Section view requires a committed STEP model")
            }
            disabled={!section.available}
            active={section.enabled}
            onClick={() => {
              section.setOpen(!section.open);
              if (!section.open) section.setEnabled(true);
            }}
          >
            <Scissors size={17} />
          </IconButton>
          <IconButton
            label={t("Fit model")}
            onClick={() => setFit((f) => f + 1)}
          >
            <Scan size={18} />
          </IconButton>
          <IconButton
            label={t("Reset camera")}
            onClick={() => {
              setPreset("iso");
              setReset((value) => value + 1);
            }}
          >
            <RotateCcw size={16} />
          </IconButton>
          <IconButton
            label={t("Toggle grid")}
            active={grid}
            onClick={() => setGrid(!grid)}
          >
            <Grid3X3 size={17} />
          </IconButton>
          <i />
          <IconButton
            label={t("Toggle orthographic projection")}
            active={ortho}
            onClick={() => setOrtho(!ortho)}
          >
            <Maximize size={16} />
          </IconButton>
          <IconButton
            label={t("Attach viewport to chat")}
            onClick={() => {
              if (capture) onScreenshot(capture());
            }}
          >
            <Camera size={17} />
          </IconButton>
          {interactive && !comparison && <PluginToolbarMenu />}
          {motion && (
            <>
              <i />
              <IconButton
                label={playing ? t("Pause motion") : t("Play motion")}
                active={playing}
                onClick={() => {
                  setMotionStarted(true);
                  setPlaying(!playing);
                }}
              >
                {playing ? <Pause size={16} /> : <Play size={16} />}
              </IconButton>
              {motionStarted && (
                <IconButton
                  label={t("Stop motion")}
                  onClick={() => {
                    setPlaying(false);
                    setMotionStarted(false);
                    setMotionReset((value) => value + 1);
                  }}
                >
                  <Square size={14} />
                </IconButton>
              )}
            </>
          )}
        </div>
        {measure && (
          <div className="measure-result">
            <Ruler size={14} />
            <Select
              aria-label={t("Measurement type")}
              value={measureKind}
              onChange={(e) => {
                setMeasureKind(e.target.value as MeasureKind);
                setPoints([]);
              }}
            >
              <option value="distance">{t("Расстояние")}</option>
              <option value="angle">{t("Угол по трём точкам")}</option>
              <option value="circle">
                {t("Радиус / диаметр по трём точкам")}
              </option>
            </Select>
            <span>{measurement(points, measureKind)}</span>
          </div>
        )}
      </div>
      <SectionControls section={section} />
      <Canvas
        onCreated={({ raycaster }) => {
          raycaster.params.Line = { threshold: 3 };
        }}
        key={String(ortho)}
        orthographic={ortho}
        camera={
          ortho
            ? { position: [180, 140, 180], zoom: 4, near: 0.1, far: 10000 }
            : { position: [180, 140, 180], fov: 38, near: 0.1, far: 10000 }
        }
        shadows
        dpr={[1, 1.75]}
        gl={{ antialias: true, preserveDrawingBuffer: true }}
        onPointerMissed={() => {
          if (interactive && !comparison)
            useWorkspace.getState().setSelected(null);
        }}
      >
        <color attach="background" args={[light ? "#f5f7f9" : "#202326"]} />
        <ambientLight intensity={1.5} />
        <hemisphereLight args={["#dbe5ff", "#4d4234", 2]} />
        <directionalLight
          position={[80, 160, 100]}
          intensity={3}
          castShadow
          shadow-mapSize={[2048, 2048]}
        />
        <directionalLight
          position={[-100, 70, -60]}
          color="#c6d9ff"
          intensity={2}
        />
        <Bounds margin={1.8} maxDuration={0.001}>
          <Model
            object={object}
            interactive={interactive}
            mode={mode}
            selected={selected}
            selectedFace={selectedFace}
            selectedEdge={selectedEdge}
            selectionMode={selectionMode}
            revisionId={comparison ? null : revisionId}
            onPoint={(p) => {
              if (interactive && measure)
                setPoints((prev) =>
                  prev.length < (measureKind === "distance" ? 2 : 3)
                    ? [...prev, p]
                    : [p],
                );
            }}
          />
          <CameraControl
            storageKey={`${viewKey}.camera${ortho ? ".orthographic" : ""}`}
            preset={preset}
            fit={fit}
            reset={reset}
            object={object}
            controlsRef={controlsRef}
            appliedPreset={appliedPreset}
          />
        </Bounds>
        {grid && <AdaptiveGrid object={object} light={light} />}
        <SectionScene
          object={object}
          plane={section.plane}
          enabled={section.clipEnabled}
          report={section.report}
          light={light}
        />
        <OrbitControls
          ref={controlsRef as never}
          makeDefault
          enableZoom={false}
          enableDamping={false}
          mouseButtons={{
            LEFT: THREE.MOUSE.ROTATE,
            MIDDLE: THREE.MOUSE.DOLLY,
            RIGHT: THREE.MOUSE.PAN,
          }}
          panSpeed={navigation.panSpeed}
          screenSpacePanning
          minDistance={3}
          maxDistance={100000}
        />
        <GizmoHelper alignment="bottom-right" margin={[56, 65]}>
          <GizmoViewport
            axisColors={["#ce726a", "#88a780", "#7a9bce"]}
            labelColor="#fff"
            hideNegativeAxes
          />
        </GizmoHelper>
        <SmoothZoom
          sensitivity={navigation.zoomSpeed}
          zoomToCursor={navigation.zoomCursor}
        />
        <OrbitSelection
          object={object}
          selected={selected}
          enabled={navigation.orbitSelection}
        />
        <Playback
          object={object}
          playing={playing}
          reset={motionReset}
          storageKey={`${viewKey}.motionTime`}
        />
        <Capture onReady={handleCapture} controlsRef={controlsRef} />
      </Canvas>
      {!object.children.length && (
        <div className="empty-scene">
          <Box size={40} />
          <h3>{t("Your next idea starts here")}</h3>
          <p>{t("Import a model or describe a part to your assistant.")}</p>
        </div>
      )}
      <div className="view-bottom">
        <div className="segmented">
          {(["iso", "top", "front", "side"] as const).map((p) => (
            <button
              key={p}
              className={preset === p ? "active" : ""}
              onClick={() => setPreset(p)}
            >
              {p === "iso"
                ? t("Isometric")
                : t(p[0].toUpperCase() + p.slice(1))}
            </button>
          ))}
        </div>
        <div className="render-select">
          <Select
            leadingIcon={<Layers size={14} />}
            aria-label={t("Rendering mode")}
            value={mode}
            onChange={(e) => setMode(e.target.value as RenderMode)}
          >
            <option value="edges">{t("Solid + edges")}</option>
            <option value="solid">{t("Solid")}</option>
            <option value="wireframe">{t("Wireframe")}</option>
            <option value="transparent">{t("Transparent")}</option>
            <option value="xray">{t("X-Ray")}</option>
            <option value="ghost">{t("Ghost")}</option>
          </Select>
        </div>
      </div>
      <div className="orbit-hint">
        <Eye size={12} /> {t("Drag to orbit")}
        <span>·</span> {t("Right button — pan")} <span>·</span>{" "}
        {t("Scroll to zoom")}
      </div>
    </div>
  );
}
