import type { TypedCadDocument } from "./typedCadDocument";
export type CadParameter = TypedCadDocument["parameters"][number];
export function parameterReferences(document: TypedCadDocument, parameterId: string): string[] {
  const references=(value:unknown):boolean=>{
    if(!value||typeof value!=="object")return false;
    if(Array.isArray(value))return value.some(references);
    const entry=value as Record<string,unknown>;
    return entry.kind==="parameter"&&entry.parameterId===parameterId || Object.values(entry).some(references);
  };
  return document.features.filter((feature)=>references(feature.operation)).map((feature)=>feature.id);
}
export function addCadParameter(document:TypedCadDocument,name:string,valueMm:number):TypedCadDocument|null {
  if(!name.trim()||/[\u0000-\u001f\u007f-\u009f]/.test(name)||name.trim().length>120||document.parameters.length>=10000||!Number.isFinite(valueMm)||Math.abs(valueMm)>10000)return null;
  const ids=new Set([...document.parameters,...document.features,...document.bodies].map((item)=>item.id));
  let index=1;while(ids.has(`parameter_${index}`))index++;
  return {...document,parameters:[...document.parameters,{id:`parameter_${index}`,name:name.trim(),valueMm}]};
}
