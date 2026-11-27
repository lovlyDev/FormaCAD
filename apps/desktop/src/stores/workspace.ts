import { create } from "zustand";
import type { Project, Parameters, Revision } from "../types";
export {currentParameters,newProject} from "./projectDefaults";
import {workspaceProjectActions} from "./workspaceProject";
import type {FolderSourceRef} from "../features/project-sources/contract";
import type {ProjectSourceBinding} from "../features/project-sources/sourceBinding";
import { persist } from "../lib/persistence";
import type { FaceSelection } from "../features/viewer/faceSelection";
import type { EdgeSelection } from "../features/viewer/edgeSelection";
export interface Workspace {
  project: Project | null;
  sourceBinding: ProjectSourceBinding | null;
  openManagedProject: (project:Project)=>void;
  openFolderProject: (project:Project,source:FolderSourceRef)=>void;
  publishFolderSnapshot: (project:Project,captured:ProjectSourceBinding,source:FolderSourceRef)=>boolean;
  selected: string | null;
  /** Ordered body selection; `selected` is its active element. */
  selectedBodies: string[];
  selectedFace: FaceSelection | null;
  selectedFaces: FaceSelection[];
  selectedEdge: EdgeSelection | null;
  selectedEdges: EdgeSelection[];
  busy: boolean;
  stage: string;
  error: string | null;
  setProject: (p: Project | null) => void;
  setSelected: (s: string | null) => void;
  setBodySelection: (bodies: readonly string[], active: string | null) => void;
  setSelectedFace: (selection: FaceSelection | null) => void;
  setFaceSelection: (selection: FaceSelection[], active: FaceSelection) => void;
  setSelectedEdge: (selection: EdgeSelection | null) => void;
  setEdgeSelection: (selection: EdgeSelection[], active: EdgeSelection) => void;
  pickSelectedEdge: (selection: EdgeSelection, additive: boolean) => void;
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
  sourceBinding: null,
  selected: null,
  selectedBodies: [],
  selectedFace: null,
  selectedFaces: [],
  selectedEdge: null,
  selectedEdges: [],
  busy: false,
  stage: "Ready",
  error: null,
  ...workspaceProjectActions(set,get),
  setSelected: (selected) => {
    const id = get().project?.id;
    if (id && get().sourceBinding?.kind !== "folder")
      persist(`forma.ui.project.${id}.selected`, JSON.stringify(selected));
    set({ selected, selectedBodies: selected ? [selected] : [], selectedFace: null, selectedFaces: [], selectedEdge: null, selectedEdges: [] });
  },
  setBodySelection: (bodies, active) => {
    const unique = [...new Set(bodies)];
    const selected = active && unique.includes(active) ? active : (unique.at(-1) ?? null);
    const id = get().project?.id;
    if (id && get().sourceBinding?.kind !== "folder") persist(`forma.ui.project.${id}.selected`, JSON.stringify(selected));
    set({ selected, selectedBodies: unique, selectedFace: null, selectedFaces: [], selectedEdge: null, selectedEdges: [] });
  },
  setSelectedFace: (selectedFace) => set({ selectedFace, selectedFaces: selectedFace ? [selectedFace] : [], selectedEdge: null, selectedEdges: [] }),
  setFaceSelection: (selectedFaces, selectedFace) => set({ selectedFaces, selectedFace, selectedEdge: null, selectedEdges: [] }),
  setSelectedEdge: (selectedEdge) => set({ selectedEdge, selectedFace: null, selectedFaces: [], selectedEdges: selectedEdge ? [selectedEdge] : [] }),
  setEdgeSelection: (selectedEdges, selectedEdge) => set({ selectedEdges, selectedEdge, selectedFace: null, selectedFaces: [] }),
  pickSelectedEdge: (edge, additive) => {
    const current = get();
    if (!additive || current.selected !== edge.bodyId) current.setSelected(edge.bodyId);
    const previous = additive ? current.selectedEdges.filter((item) =>
      item.bodyId === edge.bodyId && item.revisionId === edge.revisionId && item.sceneToken === edge.sceneToken) : [];
    const index = previous.findIndex((item) => item.edgeOrdinal === edge.edgeOrdinal);
    const next = index >= 0 ? previous.filter((_, i) => i !== index) : [...previous, edge];
    set({ selectedEdge: next.at(-1) ?? null, selectedEdges: next, selectedFace: null, selectedFaces: [] });
  },
  setBusy: (busy, stage = "Ready") => set({ busy, stage }),
  setError: (error) => set({ error }),
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
