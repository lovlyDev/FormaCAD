import { t } from "../../i18n";
import { useTheme } from "../../lib/theme";
import { persist, readState, usePersistentState } from "../../lib/persistence";
import { Canvas, useThree, useFrame } from "@react-three/fiber";
import {
  OrbitControls,
  Grid,
  GizmoHelper,
  GizmoViewport,
  Bounds,
  useBounds,
} from "@react-three/drei";
import { useCallback, useEffect, useState, useRef, type RefObject } from "react";
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
} from "lucide-react";
import { IconButton, Select } from "../../components/ui";
import { useWorkspace } from "../../stores/workspace";
import { SmoothZoom, Playback, hasMotion } from "./Playback";
import { measurement, type MeasureKind } from "../../lib/measurement";
export type RenderMode = "edges" | "solid" | "wireframe" | "transparent";
function Model({
  object,
  mode,
  selected,
  onPoint,
}: {
  object: THREE.Group;
  mode: RenderMode;
  selected: string | null;
  onPoint: (p: THREE.Vector3) => void;
}) {
  const select = useWorkspace((s) => s.setSelected);
  useEffect(() => {
    object.traverse((o) => {
      if (!(o instanceof THREE.Mesh)) return;
      const mats = Array.isArray(o.material) ? o.material : [o.material];
      mats.forEach((m) => {
        if (m instanceof THREE.MeshStandardMaterial) {
          m.wireframe = mode === "wireframe";
          m.transparent = mode === "transparent";
          m.opacity = mode === "transparent" ? 0.35 : 1;
          m.color.set(o.name === selected ? "#a78bfa" : "#b4bfc8");
        }
      });
    });
  }, [object, mode, selected]);
  useEffect(() => {
    const edges: THREE.LineSegments[] = [];
    if (mode === "edges")
      object.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          const line = new THREE.LineSegments(
            new THREE.EdgesGeometry(o.geometry, 25),
            new THREE.LineBasicMaterial({
              color: "#4a535c",
              transparent: true,
              opacity: 0.7,
            }),
          );
          o.add(line);
          edges.push(line);
        }
      });
    return () =>
      edges.forEach((e) => {
        e.removeFromParent();
        e.geometry.dispose();
        (e.material as THREE.Material).dispose();
      });
  }, [object, mode]);
  return (
    <primitive
      object={object}
      onClick={(e: {
        stopPropagation: () => void;
        object: THREE.Object3D;
        point: THREE.Vector3;
      }) => {
        e.stopPropagation();
        select(e.object.name);
        onPoint(e.point.clone());
      }}
    />
  );
}
function CameraControl({
  preset,
  fit,
  reset,
  object,
  storageKey,
  controlsRef,
  appliedPreset,
}: {
  preset: string;
  fit: number;
  reset: number;
  object: THREE.Group;
  storageKey: string;
  controlsRef: RefObject<OrbitHandle | null>;
  appliedPreset: RefObject<string>;
}) {
  const bounds = useBounds();
  const camera = useThree((state) => state.camera);
  const last = useRef<{ fit: number; reset: number; preset: string; camera: THREE.Camera } | null>(null);
  const api = useRef(bounds);
  useEffect(() => {
    api.current = bounds;
  }, [bounds]);
  useEffect(() => {
    const controls = controlsRef.current;
    if (!controls) return;
    const previous = last.current;
    last.current = { fit, reset, preset, camera };
    const saved = readState<CameraSnapshot | null>(storageKey, null);
    if ((!previous || (previous.fit === fit && previous.reset === reset && previous.preset === preset && previous.camera === camera)) && validCamera(saved)) {
      api.current.refresh(object).clip();
      camera.position.fromArray(saved.position);
      camera.quaternion.fromArray(saved.quaternion);
      camera.up.fromArray(saved.up);
      controls.target.fromArray(saved.target);
      if (camera instanceof THREE.PerspectiveCamera || camera instanceof THREE.OrthographicCamera) {
        camera.zoom = saved.zoom;
        camera.updateProjectionMatrix();
      }
      controls.update();
      return;
    }
    if (!previous || previous.fit !== fit || previous.reset !== reset || previous.camera !== camera) api.current.refresh(object).clip().fit();
    if (previous && previous.reset === reset) return;
    const positions: Record<string, [number, number, number]> = {
      iso: [180, 140, 180],
      top: [0, 260, 0.001],
      front: [0, 40, 260],
      side: [260, 40, 0],
    };
    const framing = api.current.refresh(object).clip();
    const { center } = framing.getSize();
    const distance = Math.max(camera.position.distanceTo(center), 1);
    const direction = new THREE.Vector3(
      ...(positions[preset] ?? positions.iso),
    ).normalize();
    framing
      .moveTo(center.clone().addScaledVector(direction, distance))
      .lookAt({ target: center, up: [0, 1, 0] });
    if (camera instanceof THREE.OrthographicCamera) framing.fit();
  }, [preset, object, reset, camera, controlsRef, fit, storageKey]);
  useFrame(() => {
    const controls = controlsRef.current;
    if (!controls || !last.current) return;
    if (appliedPreset.current !== preset) {
      const directions: Record<string, [number, number, number]> = {
        iso: [1, 0.8, 1],
        top: [0, 1, 0.00001],
        front: [0, 0, 1],
        side: [1, 0, 0],
      };
      const distance = Math.max(camera.position.distanceTo(controls.target), 1);
      const direction = new THREE.Vector3(
        ...(directions[preset] ?? directions.iso),
      ).normalize();
      const target = controls.target.clone();
      api.current.refresh(object).clip();
      camera.position.copy(target).addScaledVector(direction, distance);
      camera.up.set(0, preset === "top" ? 0 : 1, preset === "top" ? -1 : 0);
      camera.lookAt(target);
      controls.update();
      appliedPreset.current = preset;
    }
    const snapshot = JSON.stringify({
      position: camera.position.toArray().map(roundCamera),
      quaternion: camera.quaternion.toArray().map(roundCamera),
      up: camera.up.toArray().map(roundCamera),
      target: controls.target.toArray().map(roundCamera),
      zoom: roundCamera((camera as THREE.PerspectiveCamera).zoom),
    });
    if (localStorage.getItem(storageKey) !== snapshot) persist(storageKey, snapshot);
  });
  return null;
}
type CameraSnapshot = { position: number[]; quaternion: number[]; up: number[]; target: number[]; zoom: number };
type OrbitHandle = { target: THREE.Vector3; update: () => void };
const roundCamera = (value: number) => Math.round(value * 100_000) / 100_000;
function validCamera(value: CameraSnapshot | null): value is CameraSnapshot {
  return !!value && [value.position, value.up, value.target].every(v => Array.isArray(v) && v.length === 3 && v.every(Number.isFinite)) && Array.isArray(value.quaternion) && value.quaternion.length === 4 && value.quaternion.every(Number.isFinite) && Number.isFinite(value.zoom) && value.zoom > 0;
}
function Capture({ onReady, controlsRef }: { onReady: (fn: () => string) => void; controlsRef: RefObject<OrbitHandle | null> }) {
  const { gl, scene, camera } = useThree();
  useFrame(() => {
    if (import.meta.env.DEV)
      gl.domElement.dataset.camera = JSON.stringify({
        position: camera.position.toArray(),
        target: controlsRef.current?.target.toArray() ?? [0, 0, 0],
        zoom: camera.zoom,
      });
    if (import.meta.env.DEV) {
      const motion: { name: string; speed: number; rotation: number[] }[] = [];
      scene.traverse((node) => {
        if (node.userData.formaMotion)
          motion.push({
            name: node.name,
            speed: node.userData.formaMotion.speed,
            rotation: node.quaternion.toArray(),
          });
      });
      gl.domElement.dataset.motion = JSON.stringify(motion);
    }
  });
  useEffect(() => {
    onReady(() => {
      gl.render(scene, camera);
      return gl.domElement.toDataURL("image/png");
    });
  }, [gl, scene, camera, onReady]);
  return null;
}
export default function Viewer({
  object,
  onScreenshot,
  comparison,
}: {
  object: THREE.Group;
  onScreenshot: (data: string) => void;
  comparison?: boolean;
}) {
  const projectId = useWorkspace((s) => s.project?.id ?? "empty");
  const viewKey = `forma.ui.project.${projectId}.view.${comparison ? "comparison" : "main"}`;
  const [playing, setPlaying] = usePersistentState(`${viewKey}.playing`, false);
  const light = useTheme() === "light";
  const [motionStarted, setMotionStarted] = usePersistentState(`${viewKey}.motionStarted`, false);
  const [motionReset, setMotionReset] = useState(0);
  const motion = hasMotion(object);
  const [reset, setReset] = useState(0);
  const [mode, setMode] = usePersistentState<RenderMode>(`${viewKey}.mode`, "edges");
  const [grid, setGrid] = usePersistentState(`${viewKey}.grid`, true);
  const [preset, setPreset] = usePersistentState(`${viewKey}.preset`, "iso");
  const [ortho, setOrtho] = usePersistentState(`${viewKey}.ortho`, false);
  const [fit, setFit] = useState(0);
  const [measure, setMeasure] = useState(false);
  const [measureKind, setMeasureKind] = useState<MeasureKind>("distance");
  const [points, setPoints] = useState<THREE.Vector3[]>([]);
  const [capture, setCapture] = useState<(() => string) | null>(null);
  const controlsRef = useRef<OrbitHandle | null>(null);
  const appliedPreset = useRef(preset);
  const selected = useWorkspace((s) => s.selected);
  const handleCapture = useCallback(
    (fn: () => string) => setCapture(() => fn),
    [],
  );
  return (
    <div className="viewport">
      <div className="viewport-heading">
        <span>
          <Box size={13} />{" "}
          {comparison ? t("Revision comparison") : t("Model workspace")}
        </span>
        <span className="quiet">
          {ortho ? t("Orthographic") : t("Perspective")}{" "}
          <span className="tiny-dot" /> {t("mm")}
        </span>
      </div>
      <div className="viewport-toolbars">
        <div className="view-toolbar">
          <IconButton
            label={t("Select object")}
            active={!measure}
            onClick={() => setMeasure(false)}
          >
            <MousePointer2 size={17} />
          </IconButton>
          <IconButton
            label={t("Measure distance between two surface points")}
            active={measure}
            onClick={() => {
              setMeasure(!measure);
              setPoints([]);
            }}
          >
            <Ruler size={17} />
          </IconButton>
          <i />
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
      <Canvas
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
        onPointerMissed={() => useWorkspace.getState().setSelected(null)}
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
            mode={mode}
            selected={selected}
            onPoint={(p) => {
              if (measure)
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
        {grid && (
          <Grid
            position={[0, -0.5, 0]}
            args={[2, 2]}
            cellSize={10}
            sectionSize={50}
            cellColor={light ? "#dce3e9" : "#343a40"}
            sectionColor={light ? "#b7c6d1" : "#48525a"}
            fadeDistance={550}
            fadeStrength={1.8}
            followCamera={false}
            fadeFrom={0}
            cellThickness={0.8}
            sectionThickness={1.2}
            infiniteGrid
          />
        )}
        <OrbitControls
          ref={controlsRef as never}
          makeDefault
          enableZoom={false}
          enableDamping={false}
          mouseButtons={{ LEFT: THREE.MOUSE.ROTATE, MIDDLE: THREE.MOUSE.DOLLY, RIGHT: THREE.MOUSE.PAN }}
          panSpeed={0.3}
          screenSpacePanning
          minDistance={3}
          maxDistance={3500}
        />
        <GizmoHelper alignment="bottom-right" margin={[56, 65]}>
          <GizmoViewport
            axisColors={["#ce726a", "#88a780", "#7a9bce"]}
            labelColor="#fff"
            hideNegativeAxes
          />
        </GizmoHelper>
        <SmoothZoom />
        <Playback object={object} playing={playing} reset={motionReset} storageKey={`${viewKey}.motionTime`} />
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
          <Layers size={14} />
          <Select
            aria-label={t("Rendering mode")}
            value={mode}
            onChange={(e) => setMode(e.target.value as RenderMode)}
          >
            <option value="edges">{t("Solid + edges")}</option>
            <option value="solid">{t("Solid")}</option>
            <option value="wireframe">{t("Wireframe")}</option>
            <option value="transparent">{t("Transparent")}</option>
          </Select>
        </div>
      </div>
      <div className="orbit-hint">
        <Eye size={12} /> {t("Drag to orbit")}
        <span>·</span> {t("Right-drag to pan")} <span>·</span>{" "}
        {t("Scroll to zoom")}
      </div>
    </div>
  );
}
