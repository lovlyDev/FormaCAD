import { OffsetProfileDialog } from "../offset-profile/OffsetProfileDialog";
import { BooleanDialog } from "../boolean-tool/BooleanDialog";
import { MirrorFeaturesDialog } from "../mirror-features/MirrorFeaturesDialog";
import { ThickenDialog } from "../thicken/ThickenDialog";
import type { ThickenIntent } from "../thicken/thickenCommand";
import { SplitDialog } from "../split/SplitDialog";
import type { captureBoolean } from "../boolean-tool/booleanCommand";
import type { Project } from "../../types";
type Edit=NonNullable<ReturnType<typeof captureBoolean>>;
export function BodyOperationDialog({kind,project,bodies,initialTarget,thicken,disabled,close,apply}:{kind:"boolean"|"split"|"thicken"|"mirrorFeatures"|"offsetProfile";thicken?:ThickenIntent;project:Project;bodies:readonly{id:string;name:string}[];initialTarget:string;disabled:boolean;close:()=>void;apply:(edit:Edit,description:string)=>void}){
 if(kind==="offsetProfile")return <OffsetProfileDialog project={project} bodies={bodies} initialTarget={initialTarget} disabled={disabled} close={close} apply={edit=>apply(edit,"Offset the extrusion profile. The source profile stays linked and the distance expression is retained.")}/>;
 if(kind==="mirrorFeatures")return <MirrorFeaturesDialog project={project} bodies={bodies} initialTarget={initialTarget} disabled={disabled} close={close} apply={edit=>apply(edit,"Source operations remain linked. Mirrored material changes must leave one connected solid.")}/>;
 if(kind==="thicken")return thicken?<ThickenDialog project={project} intent={thicken} disabled={disabled} close={close} apply={edit=>apply(edit,"The original body remains. A new body is created from the selected face.")}/>:null;
 const shared={project,bodies,initialTarget,disabled,close};
 return kind==="split"?<SplitDialog {...shared} apply={edit=>apply(edit,"Split creates two named bodies in one revision.")}/>:<BooleanDialog {...shared} apply={edit=>apply(edit,"The target body changes. The tool body remains available separately.")}/>;
}
