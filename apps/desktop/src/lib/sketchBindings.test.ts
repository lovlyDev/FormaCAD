import fixture from "../../../../docs/fixtures/linked-profiles.cad.json";
import { describe,it,expect } from "vitest";
import { createLinkedProfiles } from "./templates/linkedProfiles";
import { readTypedCadDocument } from "./typedCadDocument";
import { readSketchOperation } from "./sketchDocument";
import { resolveSketchBindings,unlinkSketchBinding } from "./sketchBindings";
import { parameterReferences } from "./cadParameters";
import { removeProfilePoint } from "./sketchProfileEditing";
describe("shared profile coordinates",()=>{
 it("the actual UI template matches the native geometry fixture",()=>{
 const doc=readTypedCadDocument(createLinkedProfiles())!;
 expect(doc.features.map(feature=>feature.operation)).toEqual(fixture.features.map((feature:{operation:unknown})=>feature.operation));
 expect(doc.parameters.map(p=>({id:p.id,valueMm:p.valueMm}))).toEqual(fixture.parameters.map((p:{id:string;valueMm:number})=>({id:p.id,valueMm:p.valueMm})));
 });
 it("propagates width, depth and base height into both profiles and preserves source",()=>{
 const doc=readTypedCadDocument(createLinkedProfiles())!;
 const source=readSketchOperation(doc.features[0].operation)!,triangle=readSketchOperation(doc.features[2].operation)!;
 const changed=doc.parameters.map(p=>({...p,valueMm:({width:80,depth:30,base_height:15,triangle_height:12} as Record<string,number>)[p.id]}));
 expect(resolveSketchBindings(source,changed).points[0]).toMatchObject({xMm:-40,yMm:-15});
 const resolved=resolveSketchBindings(triangle,changed);expect(resolved.originMm).toEqual([-40,0,15]);expect(resolved.points[2].yMm).toBe(12);
 expect(source.points[0].xMm).toBe(-20);expect(parameterReferences(doc,"width")).toEqual(["base_sketch","triangle_sketch","triangle_extrude"]);
 });
 it("unlinking materializes coordinates and pruning a point removes its links",()=>{
 const doc=readTypedCadDocument(createLinkedProfiles())!,op=readSketchOperation(doc.features[0].operation)!;
 doc.parameters[0].valueMm=80;const unlinked=unlinkSketchBinding(op,doc.parameters,"a_x");
 expect(unlinked.points[0].xMm).toBe(-40);expect(unlinked.bindings).toHaveLength(7);
 const removed=removeProfilePoint(op,"a")!;expect(removed.bindings).toHaveLength(6);expect(()=>resolveSketchBindings(removed,doc.parameters)).not.toThrow();
 });
 it("rejects missing parameters and out of range coordinates",()=>{
 const doc=readTypedCadDocument(createLinkedProfiles())!,op=readSketchOperation(doc.features[0].operation)!;
 expect(()=>resolveSketchBindings(op,[])).toThrow("BROKEN_REFERENCE");doc.parameters[0].valueMm=30000;expect(()=>resolveSketchBindings(op,doc.parameters)).toThrow("INVALID_VALUE");
 });
});
