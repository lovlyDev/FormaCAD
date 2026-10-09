import { t } from "../../i18n";
export function createLinkedProfiles():string {
 const ref=(parameterId:string)=>({kind:"parameter",parameterId});
 const value=(parameterId:string,scale:number)=>({...ref(parameterId),scale,offsetMm:0});
 const point=(id:string,xMm:number,yMm:number)=>({id,xMm,yMm});
 const lines=(ids:string[])=>ids.map((id,i)=>({id:`line_${i+1}`,startPointId:id,endPointId:ids[(i+1)%ids.length]}));
 const bindings=(ids:string[],coordinates:[string,number,string,number][])=>ids.flatMap((id,i)=>[
  {id:`${id}_x`,target:"point",pointId:id,axis:"x",value:value(coordinates[i][0],coordinates[i][1])},
  {id:`${id}_y`,target:"point",pointId:id,axis:"y",value:value(coordinates[i][2],coordinates[i][3])},
 ]);
 const base=["a","b","c","d"],triangle=["left","right","ridge"];
 return JSON.stringify({schemaVersion:2,revisionId:"draft",parameters:[
  {id:"width",name:t("Linked width"),valueMm:40},{id:"depth",name:t("Linked depth"),valueMm:20},
  {id:"base_height",name:t("Base height"),valueMm:10},{id:"triangle_height",name:t("Triangle height"),valueMm:10}],
 features:[{id:"base_sketch",name:t("Linked base profile"),operation:{type:"sketch2d",plane:"xy",originMm:[0,0,0],points:[point("a",-20,-10),point("b",20,-10),point("c",20,10),point("d",-20,10)],lines:lines(base),constraints:[],bindings:bindings(base,[["width",-.5,"depth",-.5],["width",.5,"depth",-.5],["width",.5,"depth",.5],["width",-.5,"depth",.5]])}},
 {id:"base_extrude",name:t("Base extrusion"),operation:{type:"extrude",sketchId:"base_sketch",distance:ref("base_height")}},
 {id:"triangle_sketch",name:t("Linked triangle profile"),operation:{type:"sketch2d",plane:"yz",originMm:[-20,0,10],points:[point("left",-10,0),point("right",10,0),point("ridge",0,10)],lines:lines(triangle),constraints:[],bindings:[...bindings(triangle,[["depth",-.5,"triangle_height",0],["depth",.5,"triangle_height",0],["depth",0,"triangle_height",1]]),{id:"origin_x",target:"origin",axis:"x",value:value("width",-.5)},{id:"origin_z",target:"origin",axis:"z",value:value("base_height",1)}]}},
 {id:"triangle_extrude",name:t("Triangle extrusion"),operation:{type:"extrude",sketchId:"triangle_sketch",distance:ref("width")}},
 {id:"joined",name:t("Joined profiles"),operation:{type:"boolean",mode:"union",leftFeatureId:"base_extrude",rightFeatureId:"triangle_extrude"}}],bodies:[{id:"body",name:t("Linked profiles"),sourceFeatureId:"joined"}]},null,2);
}
