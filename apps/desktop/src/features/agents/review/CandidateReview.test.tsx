import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Group } from "three";
import { CandidateReview } from "./CandidateReview";
import { newProject, useWorkspace } from "../../../stores/workspace";
const mocks = vi.hoisted(() => ({ preview: vi.fn() }));
vi.mock("../../model-preview/useModelPreview", () => ({ useModelPreview: mocks.preview }));
vi.mock("./useSavedBaseline", () => ({ useSavedBaseline: () => ({ object: null, error: false, pending: false }) }));
vi.mock("../../model-preview/PreviewCanvas", () => ({ PreviewCanvas: () => <div /> }));
const source = JSON.stringify({ schemaVersion: 2, revisionId: "candidate", parameters: [], features: [], bodies: [] });
const report = { available: true, pending: false, waiting: false, preview: { object: new Group(), metrics: { volumeMm3: 10, areaMm2: 12, boundsMm: [1, 2, 5] } }, error: null };
afterEach(() => { cleanup(); vi.clearAllMocks(); });
describe("candidate acceptance", () => {
  it("rejects without accepting while preview is pending or failed", () => {
    const base = newProject("Review", "blank", "mm", "codex"); useWorkspace.getState().setProject(base);
    mocks.preview.mockReturnValue({ ...report, pending: true }); const onDecision = vi.fn();
    const mounted = render(<CandidateReview review={{ base, source }} onDecision={onDecision} />);
    expect(screen.getByRole("button", { name: "Accept candidate" })).toBeDisabled();
    mocks.preview.mockReturnValue({ ...report, error: "failure", preview: null });
    mounted.rerender(<CandidateReview review={{ base, source }} onDecision={onDecision} />);
    expect(screen.getByRole("button", { name: "Accept candidate" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Reject candidate" }));
    expect(onDecision.mock.calls).toEqual([[false]]);
    expect(useWorkspace.getState().project).toBe(base);
  });
  it("blocks acceptance when another project shares the revision or the revision changes", async () => {
    const base = newProject("Review", "blank", "mm", "codex"); useWorkspace.getState().setProject(base);
    mocks.preview.mockReturnValue(report); const onDecision = vi.fn();
    render(<CandidateReview review={{ base, source }} onDecision={onDecision} />);
    expect(screen.getByRole("button", { name: "Accept candidate" })).toBeEnabled();
    await act(async () => useWorkspace.getState().setProject({ ...base, id: "different" }));
    expect(screen.getByRole("alert")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Accept candidate" })); expect(onDecision).not.toHaveBeenCalled();
    await act(async () => useWorkspace.getState().setProject({ ...base, currentRevision: "new" }));
    expect(screen.getByRole("button", { name: "Accept candidate" })).toBeDisabled();
  });
  it("accepts only a fresh candidate and keeps comparison controls read-only", async () => {
    const base = newProject("Review", "blank", "mm", "codex"); useWorkspace.getState().setProject(base);
    mocks.preview.mockReturnValue(report); const onDecision = vi.fn();
    render(<CandidateReview review={{ base, source }} onDecision={onDecision} />);
    fireEvent.click(screen.getByRole("button", { name: "Current model" }));
    expect(screen.getByText("No saved 3D preview is available for the source revision.")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "AI candidate" }));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Accept candidate" })));
    expect(onDecision.mock.calls).toEqual([[true]]); expect(useWorkspace.getState().project).toBe(base);
  });
});
