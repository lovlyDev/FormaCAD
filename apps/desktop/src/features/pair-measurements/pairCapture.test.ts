import { expect,it } from "vitest";
import { capturePairScope,currentPairReference,pairKinds,capturePair,matchesPairReport } from "./pairCapture";
import { pairTestContext,pairTestReport } from "./pairTestContext";
it("preserves ordered authored captures and rejects foreign, residual and unreferenced selections",()=>{
 const context=pairTestContext(),scope=capturePairScope(context.project,context.bodyId,context.sceneToken,true)!;
 expect(currentPairReference(scope,null,context.face)).toEqual(context.face.topologyRef);
 expect(currentPairReference(scope,null,{...context.face,sceneToken:"retired-object"})).toBeNull();
 expect(currentPairReference(scope,null,{...context.face,topologyRef:undefined})).toBeNull();
 const capture=capturePair(scope,context.face.topologyRef!,context.second.topologyRef!,"minimumDistance")!;
 const report=pairTestReport(capture);expect(matchesPairReport(report,capture)).toBe(true);
 if(report.query.kind!=="minimumDistance")throw Error("Expected distance query");
 expect(matchesPairReport({...report,query:{...report.query,first:report.query.second,second:report.query.first}},capture)).toBe(false);
 const curved={...context.face.topologyRef!,role:"cylinder-face:side" as const};expect(pairKinds(curved,curved)).toEqual(["minimumDistance"]);
 const circular={...context.face.topologyRef!,kind:"edge" as const,role:"cylinder-edge:top" as const};expect(pairKinds(circular,circular)).toEqual(["minimumDistance"]);
});
