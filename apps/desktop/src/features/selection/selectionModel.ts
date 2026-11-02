export type SelectionKind = "body" | "face" | "edge" | "vertex";
export interface SelectionScope {
  projectId: string;
  revisionId: string;
  sceneToken: string;
}
/** key comes from the semantic BREP-to-tessellation map, never a triangle index. */
export interface SelectionTarget {
  key: string;
  bodyId: string;
  kind: SelectionKind;
  selectable: boolean;
  visible: boolean;
}
export interface SelectionState {
  scope: SelectionScope;
  keys: string[];
  active: string | null;
}
export type SelectionAction =
  "replace" | "add" | "toggle" | "subtract" | "all" | "none" | "invert";
export function sameSelectionScope(
  left: SelectionScope,
  right: SelectionScope,
): boolean {
  return (
    left.projectId === right.projectId &&
    left.revisionId === right.revisionId &&
    left.sceneToken === right.sceneToken
  );
}
export function emptySelection(scope: SelectionScope): SelectionState {
  return { scope: { ...scope }, keys: [], active: null };
}
export function updateSelection(
  state: SelectionState,
  scope: SelectionScope,
  universe: readonly SelectionTarget[],
  hits: readonly string[],
  action: SelectionAction,
  filter: SelectionKind,
): SelectionState {
  if (!sameSelectionScope(state.scope, scope)) return emptySelection(scope);
  const eligible = [
    ...new Set(
      universe
        .filter(
          (item) => item.kind === filter && item.visible && item.selectable,
        )
        .map((item) => item.key),
    ),
  ];
  const allowed = new Set(eligible),
    hitKeys = [...new Set(hits.filter((key) => allowed.has(key)))];
  const keys = new Set(state.keys.filter((key) => allowed.has(key)));
  if (
    action === "replace" ||
    action === "none" ||
    action === "all" ||
    action === "invert"
  ) {
    if (action === "invert") {
      const inverse = eligible.filter((key) => !keys.has(key));
      keys.clear();
      inverse.forEach((key) => keys.add(key));
    } else {
      keys.clear();
      (action === "all"
        ? eligible
        : action === "replace"
          ? hitKeys
          : []
      ).forEach((key) => keys.add(key));
    }
  } else
    for (const key of hitKeys) {
      if (action === "subtract" || (action === "toggle" && keys.has(key)))
        keys.delete(key);
      else keys.add(key);
    }
  const lastHit = [...hitKeys].reverse().find((key) => keys.has(key));
  const ordered = [...keys];
  const active =
    action === "none"
      ? null
      : (lastHit ??
        (state.active && keys.has(state.active)
          ? state.active
          : (ordered.at(-1) ?? null)));
  return { scope: { ...scope }, keys: ordered, active };
}
