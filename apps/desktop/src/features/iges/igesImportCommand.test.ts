import { expect, it, vi } from "vitest";
import type { Project } from "../../types";
vi.mock("./igesApi", () => ({ commitIges: vi.fn(async () => ({ id: "project", currentRevision: "next" })) }));
import { commitIges } from "./igesApi";
import { captureIgesImport } from "./igesImportCommand";
it("captures the confirmed file and revision before permission without retargeting a later mutation", async () => {
  const project = { id: "project", currentRevision: "base" } as Project;
  const input = { projectId: "project", expectedRevision: "base", name: "part.iges", data: "data:application/octet-stream;base64,QUJD" };
  const edit = captureIgesImport(project, input, "a".repeat(64))!;
  input.name = "other.iges"; input.expectedRevision = "changed";
  await edit.invoke();
  expect(vi.mocked(commitIges).mock.calls.at(-1)?.[0]).toMatchObject({ name: "part.iges", expectedRevision: "base" });
  expect(edit.captured.base).toEqual({ id: "project", currentRevision: "base" });
  expect(captureIgesImport(project, input, "a".repeat(64))).toBeNull();
});
