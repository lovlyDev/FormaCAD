import type { SceneIdentity, SceneSnapshot } from "./committedScene";

interface DisplayedObject { uuid: string; userData: { formaNativeCadPreview?: unknown } }
/** Identity of the actual committed object being displayed, never the requested head alone. */
export function committedSelectionScene<T extends DisplayedObject>(snapshot: SceneSnapshot<T>, current: SceneIdentity | null) {
  const displayed = snapshot.displayed;
  const object = snapshot.object;
  const aligned = !!current && !!displayed && current.projectId === displayed.projectId &&
    current.revisionId === displayed.revisionId && current.sourceSeal === displayed.sourceSeal;
  const interactive = aligned && snapshot.phase === "ready" && !!object?.userData.formaNativeCadPreview;
  return { token: interactive ? object!.uuid : "", interactive };
}
