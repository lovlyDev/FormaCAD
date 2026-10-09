export interface ExportBody {
  id: string;
  name: string;
}

export function exportBodies(program?: string): ExportBody[] {
  if (!program) return [];
  try {
    const source: unknown = JSON.parse(program);
    if (!source || typeof source !== "object") return [];
    const document = source as Record<string, unknown>;
    if (document.schemaVersion !== 2 || !Array.isArray(document.bodies)) return [];
    return document.bodies.flatMap((item: unknown) => {
      if (!item || typeof item !== "object") return [];
      const body = item as Record<string, unknown>;
      return typeof body.id === "string" && typeof body.name === "string"
        ? [{ id: body.id, name: body.name }]
        : [];
    });
  } catch {
    return [];
  }
}
