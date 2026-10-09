export function previewError(error:string):{message:string;targetId:string|null;detail:string} {
 try {
  const start=error.indexOf("{"),value=JSON.parse(error.slice(start)) as {code?:string;targetId?:string;message?:string};
  const message=value.code==="INVALID_SKETCH"?"Sketch profile is invalid":value.code==="SKETCH_CONSTRAINT_CONFLICT"?"Sketch constraints conflict":"CAD document is invalid";
  return {message,targetId:typeof value.targetId==="string"?value.targetId:null,detail:error};
 }catch{return {message:"Model preview failed",targetId:null,detail:error};}
}
