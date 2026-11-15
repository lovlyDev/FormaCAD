export function previewError(error:string):{message:string;targetId:string|null;detail:string} {
 try {
  const start=error.indexOf("{"),value=JSON.parse(error.slice(start)) as {code?:string;targetId?:string;message?:string};
  const messages:Record<string,string>={
   VARIABLE_FILLET_FAILED:"VARIABLE_FILLET_FAILED",VARIABLE_FILLET_CLOSED_EDGE:"VARIABLE_FILLET_CLOSED_EDGE",VARIABLE_FILLET_CHAIN_UNSUPPORTED:"VARIABLE_FILLET_CHAIN_UNSUPPORTED",
   INVALID_PROFILE:"INVALID_PROFILE",INVALID_PROFILE_OFFSET:"INVALID_PROFILE_OFFSET",PROFILE_OFFSET_TOPOLOGY_CHANGED:"PROFILE_OFFSET_TOPOLOGY_CHANGED",PROFILE_OFFSET_FAILED:"PROFILE_OFFSET_FAILED",PROFILE_EXTRUSION_FAILED:"PROFILE_EXTRUSION_FAILED",PROFILE_REVOLUTION_FAILED:"PROFILE_REVOLUTION_FAILED",
   THICKEN_FAILED:"Face thickening failed",INVALID_THICKNESS:"Thickness must be finite and nonzero",
   SPLIT_NO_INTERSECTION:"Plane does not divide the body",INVALID_SPLIT_PLANE:"Split plane is invalid",
   EMPTY_INTERSECTION:"Intersection is empty",INVALID_SKETCH:"Sketch profile is invalid",SKETCH_CONSTRAINT_CONFLICT:"Sketch constraints conflict",
   MIRROR_FEATURES_FAILED:"MIRROR_FEATURES_FAILED",MIRROR_FEATURES_NO_EFFECT:"MIRROR_FEATURES_NO_EFFECT",
   MIRROR_FEATURES_DISCONNECTED:"MIRROR_FEATURES_DISCONNECTED",MIRROR_FEATURES_REQUIRES_SOLID:"MIRROR_FEATURES_REQUIRES_SOLID",
   INVALID_FEATURE_MIRROR_PLANE:"INVALID_FEATURE_MIRROR_PLANE",
  };
  const message=messages[value.code??""]??"CAD document is invalid";
  return {message,targetId:typeof value.targetId==="string"?value.targetId:null,detail:error};
 }catch{return {message:"Model preview failed",targetId:null,detail:error};}
}
