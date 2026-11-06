import type {ProjectSourceBinding} from "../project-sources/sourceBinding";
import {sourceRefFor} from "../project-sources/sourceBinding";
import {captureSceneRead,rememberSceneRead} from "../project-sources/sceneRead";
import { useEffect, useRef, useSyncExternalStore, useState, useMemo } from "react";
import * as THREE from "three";
import { loadModel } from "../../lib/files";
import { disposeModel } from "../../lib/model";
import type { Project, Revision } from "../../types";
import { CommittedScene, sameSceneSource, type SceneIdentity } from "./committedScene";
import { committedSelectionScene } from "./committedSelectionScene";

/** Geometry never falls back to a parameter primitive while a committed file loads or fails. */
export function useCommittedScene(project: Project | null, revision: Revision | undefined, sourceBinding?:ProjectSourceBinding|null) {
  const sourceReady=sourceBinding===undefined||!!sourceBinding&&sourceBinding.projectId===project?.id;
  const sourceOpeningId=sourceReady?sourceBinding?.openingId:undefined;
  const folder=sourceReady&&project&&sourceBinding?sourceRefFor(sourceBinding,project.id):undefined;
  const [scene] = useState(() => new CommittedScene<THREE.Group>(disposeModel));
  const snapshot = useSyncExternalStore(scene.subscribe, scene.snapshot, scene.snapshot);
  const fileName = revision?.preview ?? revision?.source;
  const requiresFile = !!fileName || !!revision?.program;
  const file = project?.files.find(item => item.name === fileName);
  const inlineVersion = useRef(0);
  // Inline strings are immutable. A changed value gets a new opaque local token;
  // never copy/base64-hash a 40MB preview during camera/store renders.
  const seal = useMemo(() => file ? `${file.name}:${file.sha256 ?? "unsealed"}:${file.size}${file.data ? `:inline-${++inlineVersion.current}` : ""}` : `missing:${fileName ?? ""}`, [file?.name, file?.sha256, file?.size, file?.data, fileName]);
  const identity: SceneIdentity | null = project && revision && requiresFile && sourceReady ? { projectId: project.id, revisionId: revision.id, sourceSeal: seal,...(sourceOpeningId?{sourceOpeningId}:{}) } : null;
  const key = useMemo(() => project && revision && requiresFile && sourceReady ? JSON.stringify({ projectId: project.id, revisionId: revision.id, sourceSeal: seal,...(sourceOpeningId?{sourceOpeningId}:{}) }) : "", [project?.id, revision?.id, requiresFile, seal,sourceOpeningId,sourceReady]);
  const sceneRead=captureSceneRead(project,sourceBinding);
  const request = useRef({ sceneRead,identity, file, folder, source: revision?.source, previewName: revision?.preview });
  request.current = { sceneRead,identity, file, folder, source: revision?.source, previewName: revision?.preview };
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    const captured = request.current;
    void scene.load(captured.identity, async () => {
      if (!captured.file || !captured.identity) throw new Error("MODEL_PREVIEW_MISSING");
      const object = captured.folder?await loadModel(captured.file,captured.identity.projectId,captured.folder):await loadModel(captured.file,captured.identity.projectId);
      rememberSceneRead(object,captured.sceneRead);
      object.userData.formaNativeCadPreview ||= captured.file.name === captured.previewName && /\.st(e)?p$/i.test(captured.source ?? "");
      return object;
    });
  }, [key, retry, scene,project,folder?.commitId]);
  // Model/section restore borrowed source materials in passive effect cleanups.
  // Release ownership only after the entire passive cleanup batch has finished.
  useEffect(() => { queueMicrotask(() => scene.releaseRetired()); }, [scene, snapshot]);
  // StrictMode performs a synthetic cleanup/restart. Defer final ownership release until that can be distinguished.
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; queueMicrotask(() => { if (!mounted.current) scene.close(); }); };
  }, [scene]);
  const displayedHere = sameSceneSource(snapshot.displayed,identity);
  const aligned = displayedHere && snapshot.displayed?.revisionId === revision?.id && snapshot.target?.sourceSeal === seal;
  return { ...snapshot, selectionScene: committedSelectionScene(snapshot, identity), object: displayedHere && requiresFile ? snapshot.object : null,
    interactive: aligned && scene.interactive(), loading: !!identity && (!aligned || snapshot.phase === "loading") && snapshot.phase !== "failed",
    failed: !!identity && snapshot.phase === "failed", hasFile: requiresFile, retry: () => setRetry(value => value + 1) };
}
