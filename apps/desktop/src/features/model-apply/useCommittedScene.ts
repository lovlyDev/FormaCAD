import { useEffect, useRef, useSyncExternalStore, useState, useMemo } from "react";
import * as THREE from "three";
import { loadModel } from "../../lib/files";
import { disposeModel } from "../../lib/model";
import type { Project, Revision } from "../../types";
import { CommittedScene, type SceneIdentity } from "./committedScene";
import { committedSelectionScene } from "./committedSelectionScene";

/** Geometry never falls back to a parameter primitive while a committed file loads or fails. */
export function useCommittedScene(project: Project | null, revision: Revision | undefined) {
  const [scene] = useState(() => new CommittedScene<THREE.Group>(disposeModel));
  const snapshot = useSyncExternalStore(scene.subscribe, scene.snapshot, scene.snapshot);
  const fileName = revision?.preview ?? revision?.source;
  const requiresFile = !!fileName || !!revision?.program;
  const file = project?.files.find(item => item.name === fileName);
  const inlineVersion = useRef(0);
  // Inline strings are immutable. A changed value gets a new opaque local token;
  // never copy/base64-hash a 40MB preview during camera/store renders.
  const seal = useMemo(() => file ? `${file.name}:${file.sha256 ?? "unsealed"}:${file.size}${file.data ? `:inline-${++inlineVersion.current}` : ""}` : `missing:${fileName ?? ""}`, [file?.name, file?.sha256, file?.size, file?.data, fileName]);
  const identity: SceneIdentity | null = project && revision && requiresFile ? { projectId: project.id, revisionId: revision.id, sourceSeal: seal } : null;
  const key = useMemo(() => project && revision && requiresFile ? JSON.stringify({ projectId: project.id, revisionId: revision.id, sourceSeal: seal }) : "", [project?.id, revision?.id, requiresFile, seal]);
  const request = useRef({ identity, file, source: revision?.source, previewName: revision?.preview });
  request.current = { identity, file, source: revision?.source, previewName: revision?.preview };
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    const captured = request.current;
    void scene.load(captured.identity, async () => {
      if (!captured.file || !captured.identity) throw new Error("MODEL_PREVIEW_MISSING");
      const object = await loadModel(captured.file, captured.identity.projectId);
      object.userData.formaNativeCadPreview ||= captured.file.name === captured.previewName && /\.st(e)?p$/i.test(captured.source ?? "");
      return object;
    });
  }, [key, retry, scene]);
  // Model/section restore borrowed source materials in passive effect cleanups.
  // Release ownership only after the entire passive cleanup batch has finished.
  useEffect(() => { queueMicrotask(() => scene.releaseRetired()); }, [scene, snapshot]);
  // StrictMode performs a synthetic cleanup/restart. Defer final ownership release until that can be distinguished.
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; queueMicrotask(() => { if (!mounted.current) scene.close(); }); };
  }, [scene]);
  const displayedHere = snapshot.displayed?.projectId === project?.id;
  const aligned = displayedHere && snapshot.displayed?.revisionId === revision?.id && snapshot.target?.sourceSeal === seal;
  return { ...snapshot, selectionScene: committedSelectionScene(snapshot, identity), object: displayedHere && requiresFile ? snapshot.object : null,
    interactive: aligned && scene.interactive(), loading: !!identity && (!aligned || snapshot.phase === "loading") && snapshot.phase !== "failed",
    failed: !!identity && snapshot.phase === "failed", hasFile: requiresFile, retry: () => setRetry(value => value + 1) };
}
