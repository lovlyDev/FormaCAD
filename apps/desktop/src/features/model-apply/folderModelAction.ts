import { useWorkspace } from "../../stores/workspace";
import type { ReviewPlan } from "../agents/review/plan/reviewPlan";
import { isFolderWriteOpeningCurrent } from "../project-sources/folderWriteResult";
import type { FolderWriteAction, FolderWriteOpening } from "../project-sources/useFolderWriteAction";
import { assertApplyBase, type CapturedApply } from "./capturedApply";

export async function writeFolderModel(
  opening: FolderWriteOpening,
  captured: CapturedApply,
  reviewPlan: ReviewPlan | null | undefined,
  write: FolderWriteAction["write"],
  confirm: () => Promise<boolean>,
) {
  if (!isFolderWriteOpeningCurrent(opening) || captured.sourceOpeningId !== opening.openingId) {
    throw new Error("MODEL_APPLY_BASE_CHANGED");
  }
  assertApplyBase(useWorkspace.getState().project, captured, opening.openingId);
  await write(opening, {
    kind: "model",
    program: captured.program,
    prompt: captured.prompt,
    expectedRevision: captured.base.currentRevision,
    ...(reviewPlan === undefined ? {} : { reviewPlan }),
  }, captured.program, confirm);
}
