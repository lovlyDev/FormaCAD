import { afterEach, expect, test } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { t } from "../i18n";
import { createStarterSketch, readSketchOperation, type SketchOperation } from "../lib/sketchDocument";
import { Sketch2dEditor } from "./Sketch2dEditor";

afterEach(cleanup);

test("sketch editor keeps typed points and adds an editable length constraint", () => {
  const draft = JSON.parse(createStarterSketch());
  const initial = readSketchOperation(draft.features[0].operation)!;
  function Harness() {
    const [operation, setOperation] = useState<SketchOperation>(initial);
    return <><Sketch2dEditor operation={operation} disabled={false} onChange={setOperation} />
      <output data-testid="sketch-data">{JSON.stringify(operation)}</output></>;
  }
  render(<Harness />);
  fireEvent.click(screen.getByText(t("Edit 2D sketch")));
  fireEvent.click(screen.getAllByRole("button", { name: t("length") })[0]);
  expect(JSON.parse(screen.getByTestId("sketch-data").textContent!).constraints).toHaveLength(1);
  fireEvent.change(screen.getByLabelText(t("Length in mm")), { target: { value: "50" } });
  const updated = JSON.parse(screen.getByTestId("sketch-data").textContent!);
  expect(updated.constraints.find((item: { kind: string }) => item.kind === "length").distance.mm).toBe(50);
  expect(updated.points).toEqual(initial.points);
  fireEvent.keyDown(screen.getByLabelText(t("Sketch plane")), { key: "Enter" });
  fireEvent.click(screen.getByRole("option", { name: t("XZ") }));
  expect(JSON.parse(screen.getByTestId("sketch-data").textContent!).plane).toBe("xz");
});
