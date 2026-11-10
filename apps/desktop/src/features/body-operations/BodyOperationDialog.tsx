import { BooleanDialog } from "../boolean-tool/BooleanDialog";
import { SplitDialog } from "../split/SplitDialog";
import type { captureBoolean } from "../boolean-tool/booleanCommand";
import type { Project } from "../../types";
type Edit=NonNullable<ReturnType<typeof captureBoolean>>;
export function BodyOperationDialog({kind,project,bodies,initialTarget,disabled,close,apply}:{kind:"boolean"|"split";project:Project;bodies:readonly{id:string;name:string}[];initialTarget:string;disabled:boolean;close:()=>void;apply:(edit:Edit,description:string)=>void}){
 const shared={project,bodies,initialTarget,disabled,close};
 return kind==="split"?<SplitDialog {...shared} apply={edit=>apply(edit,"Split creates two named bodies in one revision.")}/>:<BooleanDialog {...shared} apply={edit=>apply(edit,"The target body changes. The tool body remains available separately.")}/>;
}
