import { beforeEach, expect, it, vi } from "vitest";
import { lastTransform, amendLastTransform } from "./amendApi";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const projectId = "00000000-0000-4000-8000-000000000001",
  revision = "00000000-0000-4000-8000-000000000002";
const matrix = [1, 0, 0, 10, 0, 1, 0, 0, 0, 0, 1, 0];
const status = {
  expectedRevision: revision,
  expectedGeneration: 2,
  transactionId: revision,
  baseRevision: "00000000-0000-4000-8000-000000000003",
  commands: [
    {
      commandIndex: 0,
      bodyId: "body",
      featureId: "move",
      name: "Move",
      matrix,
    },
  ],
};
beforeEach(() => {
  invoke.mockReset();
});
it("rejects malformed native matrices and unsafe generations instead of opening an editable F9 form", async () => {
  for (const value of [
    { ...status, expectedGeneration: Number.MAX_SAFE_INTEGER + 1 },
    {
      ...status,
      commands: [
        { ...status.commands[0], matrix: [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0] },
      ],
    },
    { ...status, commands: [{ ...status.commands[0], commandIndex: 7 }] },
  ]) {
    invoke.mockResolvedValueOnce(value);
    await expect(lastTransform(projectId)).rejects.toThrow(
      "AMEND_REPORT_INVALID",
    );
  }
});
it("cannot redirect a captured amendment to a copied project that shares revision IDs", async () => {
  invoke.mockResolvedValueOnce(status);
  const captured = (await lastTransform(projectId))!;
  invoke.mockClear();
  await expect(
    amendLastTransform(
      { id: "00000000-0000-4000-8000-000000000004", currentRevision: revision },
      captured,
      [{ commandIndex: 0, matrix }],
    ),
  ).rejects.toThrow("AMEND_STALE");
  await expect(
    amendLastTransform({ id: projectId, currentRevision: null }, captured, [
      { commandIndex: 0, matrix },
    ]),
  ).rejects.toThrow("AMEND_STALE");
  await expect(
    amendLastTransform({ id: projectId, currentRevision: revision }, captured, [
      { commandIndex: 0, matrix: [NaN] },
    ]),
  ).rejects.toThrow("TRANSFORM_INPUT_INVALID");
  expect(invoke).not.toHaveBeenCalled();
});
