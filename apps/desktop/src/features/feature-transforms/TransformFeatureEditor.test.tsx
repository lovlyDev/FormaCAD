import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { TypedFeatureTree } from "../../components/TypedFeatureTree";
import { setLocale, t } from "../../i18n";
import { activeFeatureIds, readTypedCadDocument } from "../../lib/typedCadDocument";
import { readTransformOperation } from "./transformOperation";

afterEach(cleanup);
const operation = { type: "rotate", bodyFeatureId: "solid", axisOriginMm: [0, 0, 0], axisDirection: [0, 0, 1], angleDeg: 90 };
const document = { schemaVersion: 2, revisionId: "revision", parameters: [], features: [
  { id: "profile", name: "Profile", operation: { type: "rectangle", width: { kind: "literal", mm: 40 }, depth: { kind: "literal", mm: 20 } } },
  { id: "solid", name: "Solid", operation: { type: "extrude", sketchId: "profile", distance: { kind: "literal", mm: 10 } } },
  { id: "rotation", name: "Rotated", operation },
], bodies: [{ id: "body", name: "Body", sourceFeatureId: "rotation" }] };
it("keeps transform sources in the active dependency chain", () => {
  expect([...activeFeatureIds(readTypedCadDocument(JSON.stringify(document))!)]).toEqual(["rotation", "solid", "profile"]);
});
it.each(["ru", "en"] as const)("edits literal degrees without changing source IDs or saving a revision (%s)", locale => {
  setLocale(locale); const onChange = vi.fn();
  const view = render(<TypedFeatureTree source={JSON.stringify(document)} disabled={false} onChange={onChange} />);
  const toggle = view.getByText(t("Edit transform"));
  if (toggle.closest("details")?.dataset.expanded !== "true") fireEvent.click(toggle);
  fireEvent.change(view.getByRole("spinbutton", { name: t("Rotation angle in degrees") }), { target: { value: "-45" } });
  const result = readTypedCadDocument(onChange.mock.calls[0][0])!;
  expect(result.revisionId).toBe("revision");
  expect(result.features[2].operation).toEqual({ ...operation, angleDeg: -45 });
  expect(result.features.slice(0, 2)).toEqual(document.features.slice(0, 2));
  expect(result.bodies).toEqual(document.bodies);
});
it("keeps a temporarily zero draft axis editable and rejects nonfinite or excessive input", () => {
  expect(readTransformOperation({ ...operation, axisDirection: [0, 0, 0] })).not.toBeNull();
  expect(readTransformOperation({ ...operation, axisOriginMm: [Infinity, 0, 0] })).toBeNull();
  expect(readTransformOperation({ ...operation, angleDeg: 361 })).toBeNull();
});
