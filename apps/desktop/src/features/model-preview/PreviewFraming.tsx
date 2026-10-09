import { useEffect,useRef,type RefObject } from "react";
import { useFrame,useThree } from "@react-three/fiber";
import * as THREE from "three";
import type { OrbitHandle } from "../viewer/CameraControl";
import { CameraFitTransition,fitPose } from "./cameraFit";
export function PreviewFraming({object,fit,controls}:{object:THREE.Group;fit:number;controls:RefObject<OrbitHandle|null>}){
 const {camera,gl,size,invalidate}=useThree();const transition=useRef<CameraFitTransition|null>(null),initialized=useRef(false),previousEnabled=useRef(true);
 const release=()=>{transition.current=null;if(controls.current)controls.current.enabled=previousEnabled.current;};
 useEffect(()=>{
  const orbit=controls.current;if(!orbit||!(camera instanceof THREE.PerspectiveCamera))return;
  const to=fitPose(camera,orbit.target,object);if(!to)return;
  if(!transition.current)previousEnabled.current=orbit.enabled;
  const reduced=window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  transition.current=new CameraFitTransition({position:camera.position.clone(),target:orbit.target.clone()},to,initialized.current&&!reduced?.38:0);
  initialized.current=true;orbit.enabled=false;invalidate();
 },[object,fit,camera,size.width,size.height,controls,invalidate]);
 useEffect(()=>{const interrupt=()=>release();gl.domElement.addEventListener("pointerdown",interrupt,true);return()=>{gl.domElement.removeEventListener("pointerdown",interrupt,true);release();};},[gl,controls]);
 useFrame((_,delta)=>{
  const active=transition.current,orbit=controls.current;
  if(import.meta.env.DEV&&orbit)gl.domElement.dataset.previewCamera=JSON.stringify({position:camera.position.toArray(),target:orbit.target.toArray(),moving:!!active});
  if(!active||!orbit)return;
  const pose=active.step(delta);camera.position.copy(pose.position);orbit.target.copy(pose.target);camera.lookAt(pose.target);
  const distance=pose.position.distanceTo(pose.target);camera.near=Math.max(.001,distance/1000);camera.far=Math.max(10000,distance*10);camera.updateProjectionMatrix();
  if(import.meta.env.DEV){gl.domElement.dataset.previewCamera=JSON.stringify({position:camera.position.toArray(),target:orbit.target.toArray(),moving:!active.done});}
  if(active.done)release();else invalidate();
 });return null;
}
