import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { Group } from "three";
import { folderSource, refreshFolderSource, type ProjectSourceBinding } from "./sourceBinding";
import { useModelPreview } from "../model-preview/useModelPreview";
import type { ModelPreview } from "../model-preview/previewApi";
const mocks = vi.hoisted(() => ({ preview: vi.fn(), cancel: vi.fn().mockResolvedValue(undefined), dispose: vi.fn() }));
vi.mock("../model-preview/previewApi", () => ({ previewModel: mocks.preview, cancelModelPreview: mocks.cancel, modelPreviewAvailable: () => true }));
vi.mock("../../lib/model", () => ({ disposeModel: mocks.dispose }));
const source = (id = "first") => folderSource("same", { sourceId: id, commitId: "commit", taskScopeId: `task-${id}` });
const report = (): ModelPreview => ({ object: new Group(), metrics: { volumeMm3: 1, areaMm2: 6, faceCount: 6, edgeCount: 12, boundsMm: [1, 1, 1] } });
afterEach(() => { cleanup(); vi.useRealTimers(); vi.resetAllMocks(); mocks.cancel.mockResolvedValue(undefined); });
it("routes request and cancellation through the captured folder reference", async () => {
  vi.useFakeTimers(); const binding = source(); mocks.preview.mockResolvedValueOnce(report());
  const hook = renderHook(() => useModelPreview("same", "program", "revision", true, 0, binding));
  await act(async () => vi.advanceTimersByTime(500));
  expect(mocks.preview.mock.calls[0][4]).toEqual(binding.kind === "folder" ? binding.source : null);
  hook.unmount(); expect(mocks.cancel).toHaveBeenCalledWith("same", binding.kind === "folder" ? binding.source : null);
});
it("hides retired same-ID geometry immediately and cancels only the retired source", async () => {
  vi.useFakeTimers(); const first = source(), second = source("second"), old = report(); mocks.preview.mockResolvedValueOnce(old);
  const hook = renderHook(({ binding }) => useModelPreview("same", "program", "revision", true, 0, binding), { initialProps: { binding: first } });
  await act(async () => vi.advanceTimersByTime(500)); expect(hook.result.current.preview).toBe(old);
  hook.rerender({ binding: second }); expect(hook.result.current.preview).toBeNull();
  expect(mocks.dispose).toHaveBeenCalledWith(old.object);
  expect(mocks.cancel).toHaveBeenCalledWith("same", first.kind === "folder" ? first.source : null);
});
it("rejects a late worker reply after same-ID reopen even with identical host reference", async () => {
  vi.useFakeTimers(); let resolve!: (value: ModelPreview) => void;
  mocks.preview.mockImplementationOnce(() => new Promise<ModelPreview>(done => { resolve = done; }));
  const hook = renderHook(({ binding }) => useModelPreview("same", "program", "revision", true, 0, binding), { initialProps: { binding: source() } });
  act(() => vi.advanceTimersByTime(500)); const mayAdopt = mocks.preview.mock.calls[0][3];
  hook.rerender({ binding: source() }); expect(mayAdopt()).toBe(false);
  const late = report(); await act(async () => resolve(late));
  expect(hook.result.current.preview).toBeNull(); expect(mocks.dispose).toHaveBeenCalledWith(late.object);
});
it("restarts strict preview IO after metadata commit advances within one opening", async () => {
  vi.useFakeTimers(); const first = source(); if (first.kind !== "folder") throw Error("fixture");
  mocks.preview.mockResolvedValueOnce(report()).mockResolvedValueOnce(report());
  const hook = renderHook(({ binding }) => useModelPreview("same", "program", "revision", true, 0, binding), { initialProps: { binding: first as ProjectSourceBinding } });
  await act(async () => vi.advanceTimersByTime(500));
  const next = refreshFolderSource(first, { ...first.source, commitId: "next" }); hook.rerender({ binding: next });
  expect(hook.result.current.preview).toBeNull(); await act(async () => vi.advanceTimersByTime(500));
  expect(mocks.preview.mock.calls[1][4]).toEqual({ ...first.source, commitId: "next" });
});
it("never falls back to managed IPC for absent or mismatched explicit binding", () => {
  vi.useFakeTimers(); const binding = source();
  const hook = renderHook(({ value }) => useModelPreview("different", "program", "revision", true, 0, value), { initialProps: { value: binding as ProjectSourceBinding | null } });
  act(() => vi.advanceTimersByTime(500)); expect(hook.result.current.available).toBe(false);
  hook.rerender({ value: null }); act(() => vi.advanceTimersByTime(500)); expect(mocks.preview).not.toHaveBeenCalled(); expect(mocks.cancel).not.toHaveBeenCalled();
});
it("does not publish a retired opening's failure into the new opening", async () => {
  vi.useFakeTimers(); let reject!: (reason: Error) => void;
  mocks.preview.mockImplementationOnce(() => new Promise<ModelPreview>((_, fail) => { reject = fail; }));
  const hook = renderHook(({ binding }) => useModelPreview("same", "program", "revision", true, 0, binding), { initialProps: { binding: source() } });
  act(() => vi.advanceTimersByTime(500)); hook.rerender({ binding: source("second") });
  await act(async () => reject(Error("old worker error")));
  expect(hook.result.current.error).toBeNull(); expect(hook.result.current.preview).toBeNull();
  const next = report(); mocks.preview.mockResolvedValueOnce(next);
  await act(async () => vi.advanceTimersByTime(500)); expect(hook.result.current.preview).toBe(next);
});
it("disabling folder preview cancels its source and disposes the late object", async () => {
  vi.useFakeTimers(); const binding = source(); let resolve!: (value: ModelPreview) => void;
  mocks.preview.mockImplementationOnce(() => new Promise<ModelPreview>(done => { resolve = done; }));
  const hook = renderHook(({ enabled }) => useModelPreview("same", "program", "revision", enabled, 0, binding), { initialProps: { enabled: true } });
  act(() => vi.advanceTimersByTime(500)); hook.rerender({ enabled: false });
  expect(mocks.cancel).toHaveBeenCalledWith("same", binding.kind === "folder" ? binding.source : null);
  const late = report(); await act(async () => resolve(late));
  expect(mocks.dispose).toHaveBeenCalledWith(late.object); expect(hook.result.current.preview).toBeNull();
});
