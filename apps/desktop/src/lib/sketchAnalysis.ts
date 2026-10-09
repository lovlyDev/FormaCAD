import { invoke, isTauri } from "@tauri-apps/api/core";
import { z } from "zod";
import type { SketchOperation } from "./sketchDocument";
import type { CadParameter } from "./cadParameters";
const analysisSchema=z.object({
  status:z.enum(["solved","conflict","invalidStructure","invalidProfile"]),errorCode:z.string().nullable(),
  degreesOfFreedom:z.number().int().nonnegative().nullable(),redundantEquations:z.number().int().nonnegative().nullable(),
  maxResidualMm:z.number().finite().nonnegative().nullable(),
  constraints:z.array(z.object({id:z.string(),residualMm:z.number().finite().nonnegative(),satisfied:z.boolean()})),
  solvedPoints:z.array(z.object({id:z.string(),xMm:z.number().finite(),yMm:z.number().finite()})),
  loopCount:z.number().int().nonnegative(),holeCount:z.number().int().nonnegative(),profileAreaMm2:z.number().finite().nonnegative().nullable(),
});
export type SketchAnalysis=z.infer<typeof analysisSchema>;
export const sketchAnalysisAvailable=()=>isTauri();
export async function analyzeSketch(operation:SketchOperation,parameters:CadParameter[],featureId:string):Promise<SketchAnalysis> {
  return analysisSchema.parse(await invoke("analyze_sketch",{operation,parameters,featureId}));
}
