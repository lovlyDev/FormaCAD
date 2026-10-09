import { type ComponentProps } from "react";
import { AnimatedDetails } from "./AnimatedDetails";
import { usePersistentState } from "../../lib/persistence";
import { useWorkspace } from "../../stores/workspace";
import "./EditorDisclosure.css";
export function useEditorPreference<T>(name:string,fallback:T){
 const projectId=useWorkspace(state=>state.project?.id??"home");
 return usePersistentState(`forma.ui.project.${projectId}.modelEditor.${name}`,fallback);
}
export function PersistentDetails({stateId,initialOpen=false,children,...props}:ComponentProps<"details">&{stateId:string;initialOpen?:boolean}){
 const [open,setOpen]=useEditorPreference(`sections.${stateId}`,initialOpen);
 return <AnimatedDetails {...props} open={open} onOpenChange={setOpen}>{children}</AnimatedDetails>;
}
