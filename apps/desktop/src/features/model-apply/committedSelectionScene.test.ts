import { expect,it } from "vitest";
import { committedSelectionScene } from "./committedSelectionScene";
import type { SceneSnapshot } from "./committedScene";
it("binds actual displayed UUID and refuses retained geometry before requested replacement",()=>{
 const identity={projectId:"project",revisionId:"head",sourceSeal:"seal"};
 const snapshot:SceneSnapshot<{uuid:string;userData:{formaNativeCadPreview:boolean}}>={phase:"ready",displayed:identity,target:identity,object:{uuid:"actual-object-a",userData:{formaNativeCadPreview:true}}};
 const first=committedSelectionScene(snapshot,identity);expect(first.interactive).toBe(true);
 expect(committedSelectionScene({...snapshot,object:{...snapshot.object!,uuid:"actual-object-b"}},identity).token).not.toBe(first.token);
 expect(committedSelectionScene(snapshot,{...identity,sourceSeal:"changed-before-effect"})).toEqual({interactive:false,token:""});
 expect(committedSelectionScene({...snapshot,phase:"loading"},identity).interactive).toBe(false);
 expect(committedSelectionScene({...snapshot,object:{uuid:"draft",userData:{formaNativeCadPreview:false}}},identity).interactive).toBe(false);
});
