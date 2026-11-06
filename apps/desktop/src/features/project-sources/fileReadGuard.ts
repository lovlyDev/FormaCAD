import type { ProjectFile } from "../../types";
import type { FolderSourceRef } from "./contract";
import { readProjectSnapshot } from "../project-catalog/api";

/** Parsing can outlive asset IO: recheck the same source/commit before publication. */
export async function verifyFileSource(projectId: string, file: ProjectFile, source: FolderSourceRef): Promise<void> {
  const snapshot = await readProjectSnapshot(projectId, source);
  const saved = snapshot.files.find(entry => entry.name === file.name);
  if (snapshot.id !== projectId || !file.sha256 || !saved || saved.sha256 !== file.sha256 || saved.size !== file.size) {
    throw new Error("PROJECT_SNAPSHOT_CHANGED");
  }
}
