import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderOpen, Star } from "lucide-react";
import { Button, IconButton } from "../../components/ui";
import { errorText, getLocale, t } from "../../i18n";
import {
  listRecentFolders, pinRecentFolder, recentFoldersKey, rememberRecentFolders, type RecentFolder,
} from "./recentFoldersApi";
import "./RecentFoldersPanel.css";

type Change = { location: string; pinned: boolean } | { remember: boolean };

export function RecentFoldersPanel({ openFolder }: { openFolder: (location: string) => void }) {
  const query = useQueryClient();
  const recents = useQuery({ queryKey: recentFoldersKey, queryFn: listRecentFolders });
  const change = useMutation({
    mutationFn: async (value: Change) => {
      if ("remember" in value) await rememberRecentFolders(value.remember);
      else if (!await pinRecentFolder(value.location, value.pinned)) throw new Error("PROJECT_FOLDER_INDEX_ENTRY_MISSING");
    },
    onSettled: () => query.invalidateQueries({ queryKey: recentFoldersKey }),
  });
  if (recents.isPending || recents.data === null) return null;
  const error = recents.error ?? change.error;
  return <section className="recent-folder-panel">
    <div className="section-heading">
      <h2>{t("Recent project folders")}</h2>
      {recents.data && <label className="recent-folder-setting">
        <input
          type="checkbox" checked={recents.data.rememberRecent} disabled={change.isPending}
          onChange={event => change.mutate({ remember: event.target.checked })}
        />
        {t("Remember opened folders")}
      </label>}
    </div>
    {error && <div className="error-inline">{errorText(error)}</div>}
    {!!recents.data?.items.length && <ul className="recent-folder-list">
      {recents.data.items.map(item => <RecentFolderRow
        key={item.location} item={item} openFolder={openFolder} disabled={change.isPending}
        pin={() => change.mutate({ location: item.location, pinned: !item.pinned })}
      />)}
    </ul>}
  </section>;
}

function RecentFolderRow({ item, openFolder, pin, disabled }: {
  item: RecentFolder;
  openFolder: (location: string) => void;
  pin: () => void;
  disabled: boolean;
}) {
  return <li className="recent-folder-row">
    <Button className="recent-folder-location" onClick={() => openFolder(item.location)}>
      <FolderOpen size={15} /><span title={item.location}>{item.location}</span>
    </Button>
    <span>{new Date(item.lastOpenedAtMs).toLocaleString(getLocale())}</span>
    <IconButton
      label={item.pinned ? t("Unpin project folder") : t("Pin project folder")}
      aria-pressed={item.pinned} active={item.pinned} onClick={pin} disabled={disabled}
    >
      <Star size={15} fill={item.pinned ? "currentColor" : "none"} />
    </IconButton>
  </li>;
}
