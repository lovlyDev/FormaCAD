export type Template =
  "box" | "blank" | "bracket" | "enclosure" | "plate" | "cylinder";
export type Agent = "codex" | "claude" | "custom";
export interface Parameters {
  kind: Template;
  width: number;
  depth: number;
  height: number;
  thickness: number;
  holeDiameter: number;
  holes: number;
}
export interface Revision {
  id: string;
  parent: string | null;
  createdAt: string;
  prompt: string;
  parameters: Parameters;
  source?: string;
  preview?: string;
  program?: string;
  programBase?: string;
}
export interface Message {
  id: string;
  role: "user" | "assistant" | "event" | "error";
  text: string;
  createdAt: string;
}
export interface Project {
  schemaVersion: 1;
  id: string;
  name: string;
  units: "mm" | "cm" | "inch";
  agent: Agent;
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
  revisions: Revision[];
  currentRevision: string | null;
  messages: Message[];
  files: ProjectFile[];
  exports: { name: string; createdAt: string }[];
}
export interface ProjectFile {
  name: string;
  size: number;
  kind: "model" | "image" | "drawing";
  data?: string;
  sha256?: string;
}
export interface Health {
  name: string;
  available: boolean;
  detail: string;
}
export interface Permission {
  id: string;
  projectId: string;
  action: string;
  reason: string;
  detail: string;
}
export const defaults: Parameters = {
  kind: "bracket",
  width: 120,
  depth: 65,
  height: 60,
  thickness: 5,
  holeDiameter: 8,
  holes: 4,
};
export const templates: { id: Template; name: string; description: string }[] =
  [
    {
      id: "box",
      name: "Solid block",
      description: "A cube or rectangular solid",
    },
    {
      id: "blank",
      name: "Blank CAD",
      description: "Start with a clean workspace",
    },
    {
      id: "bracket",
      name: "Mounting bracket",
      description: "A parametric right-angle bracket",
    },
    {
      id: "enclosure",
      name: "Enclosure",
      description: "An open housing with uniform walls",
    },
    {
      id: "plate",
      name: "Mechanical plate",
      description: "A base plate with mounting holes",
    },
    {
      id: "cylinder",
      name: "Cylindrical part",
      description: "A shaft or hollow sleeve",
    },
  ];
