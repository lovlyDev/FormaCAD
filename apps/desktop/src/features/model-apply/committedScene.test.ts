import { expect, it, vi } from "vitest";
import { CommittedScene } from "./committedScene";
it("retains the previous valid scene after failure and rejects late loads from another project", async () => {
  const dispose = vi.fn(), scene = new CommittedScene<string>(dispose);
  const base = { projectId: "A", revisionId: "base", sourceSeal: "a" };
  await scene.load(base, async () => "base-mesh");
  let fail!: (error: Error) => void;
  const pending = scene.load({ ...base, revisionId: "candidate", sourceSeal: "b" }, () => new Promise((_resolve, reject) => { fail = reject; }));
  expect(scene.snapshot().object).toBe("base-mesh"); expect(scene.interactive()).toBe(false);
  fail(Error("GLB failed")); await pending; expect(scene.snapshot().phase).toBe("failed");
  expect(scene.snapshot().displayed).toEqual(base); expect(dispose).not.toHaveBeenCalled();
  await scene.load({ ...base, revisionId: "new", sourceSeal: "c" }, async () => "new-mesh");
  expect(dispose).not.toHaveBeenCalled(); scene.releaseRetired(); expect(dispose).toHaveBeenCalledWith("base-mesh");
  let late!: (value: string) => void;
  const stale = scene.load({ ...base, revisionId: "late" }, () => new Promise(resolve => { late = resolve; }));
  await scene.load({ projectId: "B", revisionId: "base", sourceSeal: "d" }, async () => "B-mesh");
  late("late-mesh"); await stale; expect(scene.snapshot().object).toBe("B-mesh"); expect(dispose).toHaveBeenCalledWith("late-mesh");
  scene.releaseRetired(); await scene.load(null, async () => "never"); scene.releaseRetired(); expect(scene.snapshot().object).toBeNull();
});
it("never retires a cached instance that remains the displayed object", async () => {
  const dispose = vi.fn(), object = {}, scene = new CommittedScene(dispose);
  await scene.load({ projectId: "A", revisionId: "a", sourceSeal: "a" }, async () => object);
  await scene.load({ projectId: "A", revisionId: "b", sourceSeal: "b" }, async () => object);
  scene.releaseRetired(); expect(dispose).not.toHaveBeenCalled(); scene.close(); expect(dispose).toHaveBeenCalledTimes(1);
});
it("a late stale alias cannot dispose the active cache instance", async () => {
  const dispose = vi.fn(), object = {}, scene = new CommittedScene(dispose);
  await scene.load({ projectId: "A", revisionId: "a", sourceSeal: "a" }, async () => object);
  let finish!: (value: object) => void;
  const stale = scene.load({ projectId: "A", revisionId: "b", sourceSeal: "b" }, () => new Promise(resolve => { finish = resolve; }));
  await scene.load({ projectId: "A", revisionId: "c", sourceSeal: "c" }, async () => object);
  finish(object); await stale; scene.releaseRetired();
  expect(dispose).not.toHaveBeenCalled(); expect(scene.snapshot().object).toBe(object);
  scene.close(); expect(dispose).toHaveBeenCalledTimes(1);
});
