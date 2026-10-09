import { useEffect, useMemo, useRef, useState } from "react";
import { Box3, Vector3, type Group } from "three";
import { native } from "../../../lib/api";
import { usePersistentState } from "../../../lib/persistence";
import { useWorkspace } from "../../../stores/workspace";
import { sectionModel, type SectionReport } from "./sectionApi";
import { sectionNormals, worldToCad, type SectionPlane, type VectorMm } from "./sectionPlane";
import { useFaceSection } from "./face-section/useFaceSection";

const defaultNormal: VectorMm = [0, 0, 1];

export function useSectionView(object: Group, scope: string, comparison: boolean, suspended = false) {
  const project = useWorkspace(state => state.project);
  const selectedFace = useWorkspace(state => state.selectedFace);
  const revision = project?.revisions.find(revision => revision.id === project.currentRevision);
  const source = project?.files.find(file => file.name === revision?.source && /\.(step|stp)$/i.test(file.name));
  const available = native && !!source?.sha256 && !!project?.currentRevision && !comparison;
  const [open, setOpen] = usePersistentState(`${scope}.section.open`, false);
  const [enabled, setEnabled] = usePersistentState(`${scope}.section.enabled`, false);
  const [savedMode,setMode] = usePersistentState<"manual"|"selectedFace">(`${scope}.section.mode`,"manual");
  const mode=savedMode==="selectedFace"?"selectedFace":"manual";
  const [faceOffset,setFaceOffset] = usePersistentState(`${scope}.section.faceOffsetMm`,0);
  const [savedFaceAutomatic,setFaceAutomatic] = usePersistentState(`${scope}.section.faceAutomatic`,true);
  const faceAutomatic=savedFaceAutomatic!==false;
  const [savedAxis, setAxis] = usePersistentState<"xy" | "xz" | "yz" | "free">(`${scope}.section.axis`, "xy");
  const axis = ["xy", "xz", "yz", "free"].includes(savedAxis) ? savedAxis : "xy";
  const center = useMemo(() => worldToCad(new Box3().setFromObject(object).getCenter(new Vector3()).toArray() as VectorMm)
    .map(value => Math.round(value * 1e6) / 1e6) as VectorMm, [object]);
  const [offset, setOffset] = usePersistentState(`${scope}.section.offsetMm`, center[2]);
  const [savedNormal, setNormal] = usePersistentState<VectorMm>(`${scope}.section.normal`, defaultNormal);
  const normal = Array.isArray(savedNormal) && savedNormal.length === 3 && savedNormal.every(value => typeof value === "number" && Number.isFinite(value)) ? savedNormal : defaultNormal;
  const plane: SectionPlane = useMemo(() => {
    const direction = axis === "free" ? normal : sectionNormals[axis];
    const length = Math.hypot(...direction);
    return { normal: direction, originMm: direction.map(value => length ? value / length * offset : 0) as VectorMm, deflectionMm: 0.01 };
  }, [axis, normal, offset]);
  const valid = Number.isFinite(offset) && Math.abs(offset) <= 10000 && Math.hypot(...plane.normal) >= 1e-9 && plane.normal.every(value => Number.isFinite(value) && Math.abs(value) <= 1e6);
  const key = JSON.stringify({ projectId: project?.id, revision: project?.currentRevision, source: source?.sha256, plane });
  // Retained geometry keeps its GPU plane. Fresh-head native curves must wait
  // until that committed geometry is actually displayed.
  const alive = useRef(true), inFlight = useRef(false);
  const faceSection=useFaceSection(project,selectedFace,faceOffset,available&&enabled&&!suspended&&mode==="selectedFace",object.uuid,faceAutomatic,mode==="selectedFace"&&inFlight.current);
  const wantedManual=available && enabled && valid && !suspended && mode==="manual";
  const active = wantedManual && !faceSection.working;
  const latest = useRef(key); latest.current = key;
  const wanted = useRef(active); wanted.current = active;
  const [tick, setTick] = useState(0), [pending, setPending] = useState(false);
  const [result, setResult] = useState<{ key: string; data?: SectionReport; error?: unknown }>({ key: "" });
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    if (!active || inFlight.current || result.key === key) return;
    const timer = setTimeout(() => {
      inFlight.current = true; setPending(true);
      const request = JSON.parse(key) as { projectId: string; revision: string; source: string; plane: SectionPlane };
      void sectionModel(request.projectId, request.revision, request.source, request.plane).then(data => {
        if (alive.current && wanted.current && latest.current === key) setResult({ key, data });
      }, error => { if (alive.current && wanted.current && latest.current === key) setResult({ key, error }); })
        .finally(() => { inFlight.current = false; if (alive.current) { setPending(false); setTick(value => value + 1); } });
    }, 400);
    return () => clearTimeout(timer);
  }, [key, active, result.key, tick]);
  return { available: available && !suspended, suspended, enabled: enabled && available, setEnabled, open, setOpen, axis, setAxis, offset, setOffset, normal, setNormal, valid,
    mode,setMode,faceSection,faceOffset,setFaceOffset,faceAutomatic,setFaceAutomatic,units:project?.units??"mm",
    plane:mode==="selectedFace"?(faceSection.plane??plane):plane,
    clipEnabled:mode==="selectedFace"?!!faceSection.report:enabled&&available&&valid,
    pending: wantedManual && (faceSection.working || pending || result.key !== key), report:mode==="selectedFace"?faceSection.report:(active && result.key === key ? result.data : undefined), error:active && result.key === key ? result.error : null };
}
