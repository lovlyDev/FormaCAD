import { Box,Circle,PenTool,ArrowUpFromLine,Drill,Radius,Scissors,Move3d,Combine,Shapes,Rotate3d,FlipHorizontal2 } from "lucide-react";
export function FeatureIcon({type}:{type:string}){
 const Icon=({importStep:Box,rectangle:Box,circle:Circle,sketch2d:PenTool,extrude:ArrowUpFromLine,revolve:Rotate3d,hole:Drill,fillet:Radius,filletEdge:Radius,filletReferencedEdge:Radius,chamfer:Scissors,translate:Move3d,rotate:Rotate3d,mirror:FlipHorizontal2,boolean:Combine,sphere:Circle,cone:Shapes} as Record<string,typeof Box>)[type]??Shapes;
 return <Icon size={17} aria-hidden="true"/>;
}
