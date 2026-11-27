import type {Project,Parameters} from "../types";
import {defaults} from "../types";
export function currentParameters(p: Project | null): Parameters {
  return (
    p?.revisions.find((r) => r.id === p.currentRevision)?.parameters ?? {
      ...defaults,
      kind: "blank",
    }
  );
}
export function newProject(
  name: string,
  _kind: Parameters["kind"],
  units: Project["units"],
  agent: Project["agent"],
): Project {
  const now = new Date().toISOString();
  return {
    schemaVersion: 1,
    id: crypto.randomUUID(),
    name: name.trim(),
    units,
    agent,
    pinned: false,
    createdAt: now,
    updatedAt: now,
    revisions: [],
    currentRevision: null,
    messages: [],
    files: [],
    exports: [],
  };
}
