import { useEffect,useMemo,useRef } from "react";
import { useFrame,useThree } from "@react-three/fiber";
import { Box3,Vector3,type Group } from "three";
import { gridDistance,GridExtentTransition,MIN_GRID_DISTANCE } from "./gridExtent";
import { createGridMaterial } from "./gridMaterial";
import { gridScale } from "./gridScale";
export function AdaptiveGrid({object,light}:{object:Group;light:boolean}){
 const material=useMemo(createGridMaterial,[]),center=useRef(new Vector3()),distance=useRef(MIN_GRID_DISTANCE),transition=useRef<GridExtentTransition|null>(null),target=useRef(MIN_GRID_DISTANCE);const {gl}=useThree();
 useEffect(()=>()=>material.dispose(),[material]);
 useEffect(()=>{material.uniforms.cellColor.value.set(light?"#a4b4c2":"#65717e");material.uniforms.sectionColor.value.set(light?"#839aaf":"#8797a7");},[light,material]);
 useEffect(()=>{target.current=gridDistance(object);const bounds=new Box3().setFromObject(object);if(!bounds.isEmpty())bounds.getCenter(center.current);else center.current.set(0,0,0);const reduced=window.matchMedia("(prefers-reduced-motion: reduce)").matches;transition.current=new GridExtentTransition(distance.current,target.current,reduced?0:.5);},[object]);
 useFrame(({camera,size},delta)=>{
  const active=transition.current;if(active){distance.current=active.step(delta);if(active.done)transition.current=null;}
  const scale=gridScale(camera,size.height,center.current);material.uniforms.fadeDistance.value=distance.current;material.uniforms.cellSize.value=scale.cell;material.uniforms.scaleBlend.value=scale.blend;
  if(import.meta.env.DEV)gl.domElement.dataset.grid=JSON.stringify({distance:distance.current,target:target.current,moving:!!transition.current,minimum:MIN_GRID_DISTANCE,cell:scale.cell,blend:scale.blend});
 });
 return <mesh position={[0,-.5,0]} frustumCulled={false} raycast={()=>{}}><planeGeometry args={[2,2]}/><primitive object={material} attach="material"/></mesh>;
}
