import { invoke } from "@tauri-apps/api/core";

export interface RecentFolder {
  location: string;
  lastOpenedAtMs: number;
  pinned: boolean;
}

interface RecentFolders {
  items: RecentFolder[];
  rememberRecent: boolean;
}

export const recentFoldersKey = ["recent-project-folders"] as const;
export const listRecentFolders = () => invoke<RecentFolders | null>("list_recent_folders");
export const pinRecentFolder = (location: string, pinned: boolean) =>
  invoke<boolean>("pin_recent_folder", { location, pinned });
export const rememberRecentFolders = (enabled: boolean) =>
  invoke<void>("remember_recent_folders", { enabled });
