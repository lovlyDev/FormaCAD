export const editorTypes = [
  "viewport3d",
  "outliner",
  "files",
  "parameters",
  "ai",
  "properties",
  "history",
  "tasks",
  "activity",
  "logs",
] as const;
export type EditorType = (typeof editorTypes)[number];
export type Area = { kind: "area"; id: string; editor: EditorType };
export type Split = {
  kind: "split";
  id: string;
  axis: "x" | "y";
  ratio: number;
  first: LayoutNode;
  second: LayoutNode;
};
export type LayoutNode = Area | Split;
export interface WorkspaceLayout {
  layoutVersion: 1;
  root: LayoutNode;
}
export type WorkspacePreset = "model" | "inspect" | "sketch";
const area = (editor: EditorType): Area => ({
  kind: "area",
  id: `area-${editor}`,
  editor,
});
const split = (
  id: string,
  axis: Split["axis"],
  ratio: number,
  first: LayoutNode,
  second: LayoutNode,
): Split => ({ kind: "split", id, axis, ratio, first, second });

export function defaultLayout(
  preset: WorkspacePreset = "model",
): WorkspaceLayout {
  return {
    layoutVersion: 1,
    root: split(
      "main",
      "x",
      0.2,
      split("left", "y", 0.64, area("outliner"), area("files")),
      split(
        "body",
        "x",
        0.72,
        preset === "inspect"
          ? split("inspection", "y", 0.7, area("viewport3d"), area("history"))
          : area("viewport3d"),
        preset === "sketch"
          ? area("parameters")
          : area(preset === "inspect" ? "properties" : "ai"),
      ),
    ),
  };
}
export function areas(node: LayoutNode): Area[] {
  return node.kind === "area"
    ? [node]
    : [...areas(node.first), ...areas(node.second)];
}
export function mapNode(
  node: LayoutNode,
  id: string,
  change: (node: LayoutNode) => LayoutNode,
): LayoutNode {
  if (node.id === id) return change(node);
  return node.kind === "area"
    ? node
    : {
        ...node,
        first: mapNode(node.first, id, change),
        second: mapNode(node.second, id, change),
      };
}
export function splitArea(
  root: LayoutNode,
  id: string,
  axis: Split["axis"],
  newId: string,
): LayoutNode {
  const existing = areas(root);
  const editor = editorTypes.find(
    (type) => !existing.some((a) => a.editor === type),
  );
  if (!editor || existing.some((a) => a.id === newId)) return root;
  return mapNode(root, id, (node) =>
    node.kind !== "area"
      ? node
      : split(`split-${newId}`, axis, 0.5, node, {
          kind: "area",
          id: newId,
          editor,
        }),
  );
}
/** Each editor has one live instance. Selecting an occupied editor swaps its areas. */
export function changeEditor(
  root: LayoutNode,
  id: string,
  editor: EditorType,
): LayoutNode {
  const current = areas(root).find((a) => a.id === id);
  if (!current || current.editor === editor) return root;
  const other = areas(root).find((a) => a.editor === editor);
  let next = mapNode(root, id, (node) => ({ ...node, editor }) as Area);
  if (other)
    next = mapNode(
      next,
      other.id,
      (node) => ({ ...node, editor: current.editor }) as Area,
    );
  return next;
}
/** Remove a leaf and give its parent rectangle to the surviving subtree. */
export function joinArea(root: LayoutNode, id: string): LayoutNode {
  if (root.kind === "area") return root;
  if (root.first.kind === "area" && root.first.id === id) return root.second;
  if (root.second.kind === "area" && root.second.id === id) return root.first;
  return {
    ...root,
    first: joinArea(root.first, id),
    second: joinArea(root.second, id),
  };
}
export function resizeSplit(
  root: LayoutNode,
  id: string,
  ratio: number,
): LayoutNode {
  if (!Number.isFinite(ratio)) return root;
  return mapNode(root, id, (node) =>
    node.kind === "split"
      ? { ...node, ratio: Math.max(0.1, Math.min(0.9, ratio)) }
      : node,
  );
}
export function parseLayout(
  value: unknown,
  fallback = defaultLayout(),
): WorkspaceLayout {
  try {
    if (
      !value ||
      typeof value !== "object" ||
      !("layoutVersion" in value) ||
      value.layoutVersion !== 1 ||
      !("root" in value)
    )
      return fallback;
    const ids = new Set<string>(),
      editors = new Set<string>();
    let count = 0;
    function read(raw: unknown, depth: number): LayoutNode {
      if (!raw || typeof raw !== "object" || depth > 12 || ++count > 19)
        throw new Error("layout");
      const node = raw as Record<string, unknown>;
      if (
        typeof node.id !== "string" ||
        !/^[a-zA-Z0-9_-]{1,100}$/.test(node.id) ||
        ids.has(node.id)
      )
        throw new Error("id");
      ids.add(node.id);
      if (node.kind === "area") {
        if (
          !editorTypes.includes(node.editor as EditorType) ||
          editors.has(node.editor as string)
        )
          throw new Error("editor");
        editors.add(node.editor as string);
        return { kind: "area", id: node.id, editor: node.editor as EditorType };
      }
      if (
        node.kind !== "split" ||
        !["x", "y"].includes(node.axis as string) ||
        typeof node.ratio !== "number" ||
        !Number.isFinite(node.ratio) ||
        node.ratio < 0.1 ||
        node.ratio > 0.9
      )
        throw new Error("split");
      return {
        kind: "split",
        id: node.id,
        axis: node.axis as Split["axis"],
        ratio: node.ratio,
        first: read(node.first, depth + 1),
        second: read(node.second, depth + 1),
      };
    }
    return { layoutVersion: 1, root: read(value.root, 0) };
  } catch {
    return fallback;
  }
}
