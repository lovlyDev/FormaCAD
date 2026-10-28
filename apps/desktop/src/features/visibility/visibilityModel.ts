export interface VisibilityState {
  hidden: string[];
  isolated: string[] | null;
}
export type VisibilityAction = "hide" | "hideOthers" | "showAll" | "isolate";
export function bodyVisible(state: VisibilityState, bodyId: string): boolean {
  return (
    !state.hidden.includes(bodyId) &&
    (state.isolated === null || state.isolated.includes(bodyId))
  );
}
/** Isolation is a temporary filter over normal visibility, not a BREP edit or deletion. */
export function changeVisibility(
  state: VisibilityState,
  allBodyIds: readonly string[],
  selectedIds: readonly string[],
  action: VisibilityAction,
): VisibilityState {
  const all = new Set(allBodyIds),
    selected = new Set(selectedIds.filter((id) => all.has(id)));
  const hidden = new Set(state.hidden.filter((id) => all.has(id)));
  const isolated = state.isolated?.filter((id) => all.has(id)) ?? null;
  if (action === "showAll") return { hidden: [], isolated: null };
  if (!selected.size && action !== "isolate")
    return { hidden: [...hidden], isolated };
  if (action === "isolate")
    return {
      hidden: [...hidden],
      isolated: isolated === null && selected.size ? [...selected] : null,
    };
  for (const id of all)
    if (action === "hide" ? selected.has(id) : !selected.has(id))
      hidden.add(id);
  return { hidden: [...hidden], isolated };
}
export function readVisibility(
  value: unknown,
  allBodyIds: readonly string[],
): VisibilityState {
  if (!value || typeof value !== "object")
    return { hidden: [], isolated: null };
  const input = value as Partial<VisibilityState>,
    allowed = new Set(allBodyIds);
  const safe = (list: unknown) =>
    Array.isArray(list) && list.length <= 100000
      ? [
          ...new Set(
            list.filter(
              (item): item is string =>
                typeof item === "string" && allowed.has(item),
            ),
          ),
        ]
      : [];
  const isolated = Array.isArray(input.isolated) ? safe(input.isolated) : [];
  return {
    hidden: safe(input.hidden),
    isolated: isolated.length ? isolated : null,
  };
}
