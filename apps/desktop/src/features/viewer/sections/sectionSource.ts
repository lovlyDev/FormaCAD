import {useWorkspace} from "../../../stores/workspace";
import type {Project} from "../../../types";
import {captureSceneRead,displayedSceneRead} from "../../project-sources/sceneRead";
import type {ProjectSourceBinding} from "../../project-sources/sourceBinding";

/** Retained geometry may keep its local clipping plane in its own opening.
 * Exact new curves require the current displayed geometry and original source. */
export function captureSectionSource(object:object,project:Project|null,binding:ProjectSourceBinding|null){
 const read=captureSceneRead(project,binding),displayed=displayedSceneRead(object);
 if(!read||!binding||(!displayed&&binding.kind==="folder"))return null;
 const isOpeningCurrent=()=>read.isOpeningCurrent()&&(!displayed||displayed.isOpeningCurrent());
 const isCurrent=()=>read.isCurrent()&&(!displayed||displayed.isCurrent());
 return{openingId:binding.openingId,source:read.source,isOpeningCurrent,isCurrent,
  isSnapshotCurrent:()=>{
   const current=useWorkspace.getState().sourceBinding;
   return isCurrent()&&(!read.source||(current?.kind==="folder"&&current.source.commitId===read.source.commitId));
  }};
}
