import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { TypedFeatureTree } from "./TypedFeatureTree";
import { readTypedCadDocument } from "../lib/typedCadDocument";

afterEach(cleanup);

const source = JSON.stringify({
  schemaVersion: 2,
  revisionId: "revision_1",
  parameters: [],
  features: [
    { id: "sketch", name: "Profile", operation: { type: "circle", radius: { kind: "literal", mm: 5 } } },
    { id: "pad", name: "Pad", operation: { type: "extrude", sketchId: "sketch", distance: { kind: "literal", mm: 10 } } },
    { id: "round", name: "Round", operation: { type: "fillet", bodyFeatureId: "pad", radius: { kind: "literal", mm: 1 } } },
  ],
  bodies: [{ id: "body", name: "Body", sourceFeatureId: "round" }],
});

it("suppresses a modifier without losing its dimensions or IDs", () => {
  const onChange = vi.fn();
  const view = render(<TypedFeatureTree source={source} disabled={false} onChange={onChange} />);
  fireEvent.click(view.getByRole("checkbox"));
  const document = readTypedCadDocument(onChange.mock.calls[0][0]);
  expect(document?.features[2]).toMatchObject({
    id: "round", suppressed: true,
    operation: { type: "fillet", bodyFeatureId: "pad", radius: { kind: "literal", mm: 1 } },
  });
  expect(document?.bodies[0].sourceFeatureId).toBe("round");
});

it("rolls the body output back while retaining later features", () => {
  const onChange = vi.fn();
  const view = render(<TypedFeatureTree source={source} disabled={false} onChange={onChange} />);
  fireEvent.keyDown(view.getByRole("combobox"), { key: "Enter" });
  fireEvent.click(view.getByRole("option", { name: "Pad" }));
  const document = readTypedCadDocument(onChange.mock.calls[0][0]);
  expect(document?.bodies[0].sourceFeatureId).toBe("pad");
  expect(document?.features.map((feature) => feature.id)).toEqual(["sketch", "pad", "round"]);
  view.rerender(<TypedFeatureTree source={onChange.mock.calls[0][0]} disabled={false} onChange={onChange} />);
  expect(view.getByText("After rollback")).toBeTruthy();
});
