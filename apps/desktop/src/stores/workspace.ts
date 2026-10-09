import { create } from "zustand";
import type { Project, Parameters, Revision } from "../types";
import { defaults } from "../types";
import { saveProject } from "../lib/api";
import { persist, readState } from "../lib/persistence";
import type { FaceSelection } from "../features/viewer/faceSelection";
import type { EdgeSelection } from "../features/viewer/edgeSelection";
interface Workspace {
  project: Project | null;
  selected: string | null;
  selectedFace: FaceSelection | null;
  selectedEdge: EdgeSelection | null;
  busy: boolean;
  stage: string;
  error: string | null;
  setProject: (p: Project | null) => void;
  setSelected: (s: string | null) => void;
  setSelectedFace: (selection: FaceSelection | null) => void;
  setSelectedEdge: (selection: EdgeSelection | null) => void;
  setBusy: (b: boolean, stage?: string) => void;
  setError: (e: string | null) => void;
  update: (p: Project) => Promise<void>;
  revise: (
    parameters: Parameters,
    prompt: string,
    source?: string,
    preview?: string,
    program?: string,
    programBase?: string,
  ) => Promise<void>;
}
export const useWorkspace = create<Workspace>((set, get) => ({
  project: null,
  selected: null,
  selectedFace: null,
  selectedEdge: null,
  busy: false,
  stage: "Ready",
  error: null,
  setProject: (project) => {
    const current = get();
    const same = !!project && project.id === current.project?.id && project.currentRevision === current.project.currentRevision;
    set({
      project,
      selected: same ? current.selected : project
        ? readState(`forma.ui.project.${project.id}.selected`, null)
        : null,
      selectedFace: same ? current.selectedFace : null,
      selectedEdge: same ? current.selectedEdge : null,
      error: null,
    });
  },
  setSelected: (selected) => {
    const id = get().project?.id;
    if (id)
      persist(`forma.ui.project.${id}.selected`, JSON.stringify(selected));
    set({ selected, selectedFace: null, selectedEdge: null });
  },
  setSelectedFace: (selectedFace) => set({ selectedFace, selectedEdge: null }),
  setSelectedEdge: (selectedEdge) => set({ selectedEdge, selectedFace: null }),
  setBusy: (busy, stage = "Ready") => set({ busy, stage }),
  setError: (error) => set({ error }),
  update: async (project) => {
    const saved = await saveProject(project);
    // A delayed metadata save must not switch the user back to its original project.
    if (get().project?.id !== saved.id) return;
    set({
      project: saved,
      selectedFace:
        saved.id === get().project?.id && saved.currentRevision === get().project?.currentRevision
          ? get().selectedFace
          : null,
      selectedEdge:
        saved.id === get().project?.id && saved.currentRevision === get().project?.currentRevision
          ? get().selectedEdge
          : null,
    });
  },
  revise: async (parameters, prompt, source, preview, program, programBase) => {
    const p = get().project;
    if (!p) throw new Error("No project is open.");
    const rev: Revision = {
      id: crypto.randomUUID(),
      parent: p.currentRevision,
      createdAt: new Date().toISOString(),
      prompt,
      parameters,
      source,
      preview,
      program,
      programBase,
    };
    await get().update({
      ...p,
      currentRevision: rev.id,
      updatedAt: rev.createdAt,
      revisions: [...p.revisions, rev],
      messages: p.messages,
    });
  },
}));
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
