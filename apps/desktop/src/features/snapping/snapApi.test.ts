import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
const { exactSnap } = await import("./snapApi");

const reference = {
  schemaVersion: 1 as const,
  kind: "face" as const,
  ownerFeatureId: "pad",
  occurrencePath: [],
  role: "box-face:zmax" as const,
};
const scope = { projectId: "p", revisionId: "r", bodyId: "body" };
const query = {
  kind: "face" as const,
  reference,
  rayOriginMm: [1, 2, 50] as [number, number, number],
  rayDirection: [0, 0, -1] as [number, number, number],
};
const report = (overrides: Record<string, unknown> = {}) => ({
  projectId: "p",
  revisionId: "r",
  bodyId: "body",
  engine: { name: "occt", protocolVersion: 1 },
  evaluationSource: "rebuiltAuthoredBody",
  // Host field order differs from the request; comparison must not depend on it.
  query: {
    kind: "face",
    reference: {
      role: "box-face:zmax",
      kind: "face",
      ownerFeatureId: "pad",
      occurrencePath: [],
      schemaVersion: 1,
    },
    rayOriginMm: [1, 2, 50],
    rayDirection: [0, 0, -1],
  },
  result: { pointMm: [1, 2, 10], normal: [0, 0, 1] },
  ...overrides,
});

describe("exact snap adapter", () => {
  beforeEach(() => invoke.mockReset());
  it("accepts a verified OCCT point for the requested revision", async () => {
    invoke.mockResolvedValue(report());
    await expect(exactSnap(scope, query)).resolves.toEqual({
      pointMm: [1, 2, 10],
      normal: [0, 0, 1],
    });
    expect(invoke).toHaveBeenCalledWith(
      "snap_model_reference",
      expect.objectContaining({ expectedRevision: "r", bodyId: "body" }),
    );
  });
  it("rejects another revision, body, reference or a face without normal", async () => {
    for (const forged of [
      report({ revisionId: "other" }),
      report({ bodyId: "other" }),
      report({
        query: {
          kind: "face",
          reference: { ...reference, role: "box-face:zmin" },
        },
      }),
      report({ result: { pointMm: [1, 2, 10] } }),
      report({ engine: { name: "mesh" } }),
    ]) {
      invoke.mockResolvedValue(forged);
      await expect(exactSnap(scope, query)).rejects.toThrow(
        "SNAP_REPORT_INVALID",
      );
    }
  });
});
