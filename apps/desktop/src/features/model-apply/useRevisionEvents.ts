import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { native, listProjects } from "../../lib/api";
import { useWorkspace } from "../../stores/workspace";
import type { Project } from "../../types";

/** Events announce identities; only an authoritative snapshot can update a workspace. */
export function useRevisionEvents() {
  const query = useQueryClient();
  useEffect(() => {
    if (!native) return;
    let alive = true, stop: (() => void) | undefined;
    const generations = new Map<string, number>();
    void listen<string>("project://revision-created", ({ payload: id }) => {
      if (typeof id !== "string" || !/^[A-Za-z0-9_-]{1,80}$/.test(id)) return;
      const epoch = (generations.get(id) ?? 0) + 1; generations.set(id, epoch);
      const before = useWorkspace.getState().project;
      const cachedBefore = query.getQueryData<Project[]>(["projects"])?.find(project => project.id === id);
      void listProjects().then(projects => {
        if (!alive || generations.get(id) !== epoch) return;
        const current = useWorkspace.getState().project;
        // A newer invoke result or project switch during this fetch wins.
        if (current !== before || query.getQueryData<Project[]>(["projects"])?.find(project => project.id === id) !== cachedBefore) {
          void query.invalidateQueries({ queryKey: ["projects"] }); return;
        }
        const latest = projects.find(project => project.id === id);
        if (!latest) return;
        query.setQueryData<Project[]>(["projects"], cached => {
          const existing = cached?.find(project => project.id === id);
          if (existing !== cachedBefore) return cached;
          return existing ? cached!.map(project => project.id === id ? latest : project) : [...(cached ?? []), latest];
        });
        if (current?.id === id) useWorkspace.getState().setProject(latest);
      }).catch(cause => console.warn("Revision snapshot could not be refreshed", cause));
      void query.invalidateQueries({ queryKey: ["model-history", id] });
    }).then(unlisten => { if (!alive) unlisten(); else stop = unlisten; })
      .catch(cause => console.warn("Revision listener could not be registered", cause));
    return () => { alive = false; stop?.(); };
  }, [query]);
}
