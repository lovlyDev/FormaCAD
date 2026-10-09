import { acquireProjectAccess, releaseProjectAccess } from "./projectAccessApi";
const queues = new Map<string, Promise<unknown>>();
const users = new Map<string, number>();
const timers = new Map<string, ReturnType<typeof setTimeout>>();
function serial<T>(id: string, action: () => Promise<T>): Promise<T> {
  const result = (queues.get(id) ?? Promise.resolve()).catch(() => {}).then(action);
  queues.set(id, result);
  void result.finally(() => { if (queues.get(id) === result) queues.delete(id); }).catch(() => {});
  return result;
}
export function acquireAccess(id: string) { return serial(id, () => acquireProjectAccess(id)); }
export function retainAccess(id: string, onReleaseError: (error: unknown) => void) {
  const timer = timers.get(id); if (timer) clearTimeout(timer); timers.delete(id);
  users.set(id, (users.get(id) ?? 0) + 1);
  return () => {
    const count = Math.max(0, (users.get(id) ?? 1) - 1); users.set(id, count);
    if (count) return;
    // React StrictMode remounts must not release a live view's session lease.
    timers.set(id, setTimeout(() => {
      timers.delete(id);
      if (users.get(id)) return;
      users.delete(id);
      void serial(id, () => releaseProjectAccess(id)).catch(onReleaseError);
    }, 250));
  };
}
