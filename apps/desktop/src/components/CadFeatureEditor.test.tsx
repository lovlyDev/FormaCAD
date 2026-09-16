import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CadFeatureEditor } from "./CadFeatureEditor";
import { readCadDocument } from "../lib/cadDocument";

it("edits a parameter while preserving feature identity and downstream references", () => {
  const source = JSON.stringify({
    version: 1,
    features: [
      { id: "Sketch", operation: { type: "circle", radius: 5 } },
      {
        id: "Pad",
        operation: { type: "extrude", sketch: "Sketch", distance: 10 },
      },
    ],
    output: "Pad",
  });
  const onChange = vi.fn();
  render(
    <CadFeatureEditor source={source} disabled={false} onChange={onChange} />,
  );
  const input = screen.getByLabelText("Sketch.radius");
  fireEvent.change(input, { target: { value: "7" } });
  fireEvent.blur(input);
  const updated = readCadDocument(onChange.mock.calls[0][0]);
  expect(updated?.features[0]).toEqual({
    id: "Sketch",
    operation: { type: "circle", radius: 7 },
  });
  expect(updated?.features[1].operation).toEqual({
    type: "extrude",
    sketch: "Sketch",
    distance: 10,
  });
  expect(updated?.output).toBe("Pad");
});

it("does not reinterpret legacy Python as a document", () => {
  expect(readCadDocument("result = cq.Workplane('XY').box(1,1,1)")).toBeNull();
});
