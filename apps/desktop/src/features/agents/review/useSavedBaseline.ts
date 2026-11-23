import { useEffect, useState } from "react";
import type { Group } from "three";
import { loadModel } from "../../../lib/files";
import { disposeModel } from "../../../lib/model";
import {sourceRefFor,type ProjectSourceBinding} from "../../project-sources/sourceBinding";
import type { Project } from "../../../types";

/** Separate original-source GLB; a retired opening never lends its scene to review. */
export function useSavedBaseline(base: Project,binding?:ProjectSourceBinding|null) {
  const name=base.revisions.find(revision=>revision.id===base.currentRevision)?.preview;
  const file=base.files.find(file=>file.name===name&&/\.glb$/i.test(file.name));
  const valid=binding===undefined||!!binding&&binding.projectId===base.id;
  const folder=valid&&binding?sourceRefFor(binding,base.id):undefined;
  const key=JSON.stringify([base.id,base.currentRevision,binding?.openingId,folder?.sourceId,folder?.taskScopeId,folder?.commitId,file?.name,file?.sha256,file?.size,valid]);
  const [state,setState]=useState<{key:string;object:Group|null;error:boolean;pending:boolean}>({key:"",object:null,error:false,pending:!!file&&valid});
  useEffect(()=>{
    let active=true,owned:Group|null=null;
    setState({key,object:null,error:false,pending:!!file&&valid});
    if(file&&valid)void (folder?loadModel(file,base.id,folder):loadModel(file,base.id)).then(object=>{
      if(!active){disposeModel(object);return;}
      owned=object;setState({key,object,error:false,pending:false});
    },()=>{if(active)setState({key,object:null,error:true,pending:false});});
    return()=>{active=false;if(owned)disposeModel(owned);};
  },[key]);
  return state.key===key?state:{object:null,error:false,pending:!!file&&valid};
}
