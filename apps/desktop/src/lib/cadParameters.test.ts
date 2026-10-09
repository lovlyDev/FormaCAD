import { describe,expect,it } from "vitest";
import { addCadParameter,parameterReferences } from "./cadParameters";
import { createStarterSketch } from "./sketchDocument";
import type { TypedCadDocument } from "./typedCadDocument";
const starter=():TypedCadDocument=>JSON.parse(createStarterSketch());
describe("named parameters",()=>{
 it("adds stable IDs without touching features and rejects invalid names and numbers",()=>{
  const document=starter(),next=addCadParameter(document," Width ",8)!;
  expect(next.parameters.at(-1)).toMatchObject({name:"Width",valueMm:8});expect(next.features).toBe(document.features);
  expect(addCadParameter(document," ",1)).toBeNull();expect(addCadParameter(document,"bad\nname",1)).toBeNull();expect(addCadParameter(document,"x",Infinity)).toBeNull();
  const second=addCadParameter(next,"Height",6)!;expect(second.parameters.at(-1)!.id).not.toBe(next.parameters.at(-1)!.id);
 });
 it("finds operation references, including sketch dimensions, without treating text as a reference",()=>{
  const document=starter();document.features[0].operation={...document.features[0].operation,constraints:[{id:"length",kind:"length",lineId:"line_ab",distance:{kind:"parameter",parameterId:"width"}}]};
  expect(parameterReferences(document,"width")).toContain(document.features[0].id);expect(parameterReferences(document,"missing")).toEqual([]);
 });
});
