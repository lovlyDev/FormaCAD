import { describe, expect, it } from "vitest";
import { folderSource, isSourceCurrent, managedSource, refreshFolderSource, sourceRefFor } from "./sourceBinding";

const ref = (id = "source-a", commitId = "commit-a") => ({ sourceId: id, commitId, taskScopeId: `group-${id}` });
describe("renderer source binding foundation", () => {
  it("rejects a late result after equal-ID folder switch", async () => {
    const captured = folderSource("shared-project", ref());
    let active = captured;
    let finish!: (value: number) => void;
    const pending = new Promise<number>(resolve => { finish = resolve; }).then(value => isSourceCurrent(active, captured) ? value : null);
    active = folderSource("shared-project", ref("source-b"));
    finish(8000);
    expect(await pending).toBeNull();
  });
  it("rejects capture-close-reopen even if the supplied reference strings match", () => {
    const captured = folderSource("project", ref());
    expect(isSourceCurrent(null, captured)).toBe(false);
    expect(isSourceCurrent(folderSource("project", ref()), captured)).toBe(false);
    expect(isSourceCurrent(managedSource("project"), captured)).toBe(false);
  });
  it("keeps queued geometry compatible with metadata commit refresh through one opening", () => {
    const captured = folderSource("project", ref());
    const active = refreshFolderSource(captured, ref("source-a", "metadata-commit"));
    expect(isSourceCurrent(active, captured)).toBe(true);
    expect(sourceRefFor(captured, "project")?.commitId).toBe("commit-a");
    expect(sourceRefFor(active, "project")?.commitId).toBe("metadata-commit");
    expect(() => refreshFolderSource(active, ref("other"))).toThrow("PROJECT_SNAPSHOT_CHANGED");
  });
  it("copies and freezes references without freezing or retaining mutable caller input", () => {
    const input = ref();
    const binding = folderSource("project", input);
    input.sourceId = "changed";
    input.commitId = "changed";
    expect(sourceRefFor(binding, "project")).toEqual(ref());
    expect(Object.isFrozen(binding)).toBe(true);
    expect(Object.isFrozen(sourceRefFor(binding, "project"))).toBe(true);
  });
  it("preserves a managed publication family but gives explicit activation a fresh opening", () => {
    const captured = managedSource("project");
    expect(managedSource("project", captured)).toBe(captured);
    expect(isSourceCurrent(managedSource("project"), captured)).toBe(false);
    expect(isSourceCurrent(managedSource("other", captured), captured)).toBe(false);
    expect(sourceRefFor(captured, "project")).toBeUndefined();
  });
  it("refuses wrong project IDs instead of falling back to managed storage", () => {
    expect(() => sourceRefFor(folderSource("first", ref()), "second")).toThrow("PROJECT_SNAPSHOT_CHANGED");
    expect(() => sourceRefFor(managedSource("first"), "second")).toThrow("PROJECT_SNAPSHOT_CHANGED");
  });
});
