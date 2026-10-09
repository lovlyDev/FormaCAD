import { expect, it } from "vitest";
import { readTopologyReference } from "./topologyReference";
import { matchesReferenceRoute } from "./referenceRoute";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";

const reference = { schemaVersion: 1 as const, kind: "edge" as const, ownerFeatureId: "pad", role: "box-edge:x:ymin:zmax" as const, occurrencePath: ["moved", "turned"] };
const document = readTypedCadDocument(JSON.stringify({ schemaVersion: 2, revisionId: "revision", parameters: [], features: [
  { id: "profile", name: "Profile", operation: { type: "rectangle" } },
  { id: "pad", name: "Pad", operation: { type: "extrude", sketchId: "profile" } },
  { id: "moved", name: "Moved", operation: { type: "translate", bodyFeatureId: "pad" } },
  { id: "turned", name: "Turned", operation: { type: "rotate", bodyFeatureId: "moved" } },
  { id: "second", name: "Second", operation: { type: "mirror", bodyFeatureId: "pad" } },
], bodies: [{ id: "body", name: "Body", sourceFeatureId: "turned" }] }))!;
it("keeps source ownership and occurrence path instead of accepting a lookalike branch", () => {
  expect(readTopologyReference(reference)).toEqual(reference);
  expect(matchesReferenceRoute(document, "turned", reference)).toBe(true);
  expect(matchesReferenceRoute(document, "second", reference)).toBe(false);
  expect(matchesReferenceRoute(document, "turned", { ...reference, ownerFeatureId: "second" })).toBe(false);
  expect(matchesReferenceRoute({ ...document, features: [...document.features].reverse() }, "turned", reference)).toBe(true);
});
it("rejects mismatched kinds, role injection and cyclic or excessive occurrence paths", () => {
  for (const invalid of [
    { ...reference, kind: "face" }, { ...reference, role: "nearest-edge" }, { ...reference, sourcePath: "C:/external.step" },
    { ...reference, occurrencePath: ["pad"] }, { ...reference, occurrencePath: ["moved", "moved"] },
    { ...reference, occurrencePath: Array.from({ length: 65 }, (_, i) => `transform_${i}`) },
  ]) expect(readTopologyReference(invalid)).toBeNull();
});
it("accepts only constructor-owned circular roles on the authored circle route", () => {
  const cylinder={...document,features:document.features.map(feature=>feature.id==="profile"?{...feature,operation:{type:"circle",radius:{kind:"literal",mm:10}}}:feature)};
  const circular={...reference,role:"cylinder-edge:top" as const};
  expect(readTopologyReference(circular)).toEqual(circular);
  expect(matchesReferenceRoute(cylinder,"turned",circular)).toBe(true);
  expect(matchesReferenceRoute(document,"turned",circular)).toBe(false);
  expect(matchesReferenceRoute(cylinder,"turned",reference)).toBe(false);
  expect(readTopologyReference({...circular,role:"cylinder-edge:seam"})).toBeNull();
  expect(readTopologyReference({...circular,kind:"face"})).toBeNull();
  expect(readTopologyReference({...circular,kind:"face",role:"cylinder-face:side"})).not.toBeNull();
  expect(matchesReferenceRoute({...cylinder,features:cylinder.features.map(feature=>feature.id==="profile"?{...feature,suppressed:true}:feature)},"turned",circular)).toBe(false);
});
