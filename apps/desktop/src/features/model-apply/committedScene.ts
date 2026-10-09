/** Atomic committed geometry loading; renderer owns final disposal timing. */
export interface SceneIdentity { projectId: string; revisionId: string; sourceSeal: string }
export interface SceneSnapshot<T> {
  phase: "empty" | "loading" | "ready" | "failed";
  target: SceneIdentity | null;
  displayed: SceneIdentity | null;
  object: T | null;
  cause?: unknown;
}
const sameScene = (a: SceneIdentity | null, b: SceneIdentity | null) => !!a && !!b && a.projectId === b.projectId && a.revisionId === b.revisionId && a.sourceSeal === b.sourceSeal;

export class CommittedScene<T> {
  private value: SceneSnapshot<T> = { phase: "empty", target: null, displayed: null, object: null };
  private listeners = new Set<() => void>();
  private retired: T[] = [];
  private epoch = 0;
  private pending: AbortController | null = null;
  constructor(private dispose: (object: T) => void) {}
  snapshot = () => this.value;
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  private publish(value: SceneSnapshot<T>) { this.value = value; this.listeners.forEach(listener => listener()); }
  private retire(object: T | null) { if (object !== null && !this.retired.includes(object)) this.retired.push(object); }
  /** Call after React commits the replacement; not before current object leaves its scene. */
  releaseRetired() { const items = this.retired.splice(0); items.forEach(object => this.dispose(object)); }
  interactive() { return this.value.phase === "ready" && sameScene(this.value.target, this.value.displayed); }

  async load(target: SceneIdentity | null, load: (signal: AbortSignal) => Promise<T>): Promise<void> {
    if (target && this.value.phase === "ready" && sameScene(this.value.displayed, target)) return;
    const epoch = ++this.epoch;
    this.pending?.abort(); this.pending = null;
    const retained = target && this.value.displayed?.projectId === target.projectId ? this.value.object : null;
    if (retained === null) this.retire(this.value.object);
    if (!target) { this.publish({ phase: "empty", target: null, displayed: null, object: null }); return; }
    const captured = Object.freeze({ ...target });
    const abort = new AbortController(); this.pending = abort;
    this.publish({ phase: "loading", target: captured, displayed: retained === null ? null : this.value.displayed, object: retained });
    try {
      const object = await load(abort.signal);
      if (epoch !== this.epoch || abort.signal.aborted) {
        if (object !== this.value.object && !this.retired.includes(object)) this.dispose(object);
        return;
      }
      if (this.value.object !== object) this.retire(this.value.object);
      this.pending = null;
      this.publish({ phase: "ready", target: captured, displayed: captured, object });
    } catch (cause) {
      if (epoch !== this.epoch || abort.signal.aborted) return;
      this.pending = null;
      this.publish({ ...this.value, phase: "failed", cause });
    }
  }
  /** Unmount: late load results are disposed by their epoch guard. */
  close() {
    this.epoch++; this.pending?.abort(); this.pending = null;
    this.retire(this.value.object);
    this.value = { phase: "empty", target: null, displayed: null, object: null };
    this.releaseRetired(); this.listeners.clear();
  }
}
