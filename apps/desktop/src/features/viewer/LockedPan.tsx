import { useEffect,type RefObject } from "react";
import { useThree } from "@react-three/fiber";
import type { OrbitHandle } from "./CameraControl";
import { attachLockedPan,type PanAnchor } from "./lockedPanGesture";
import { panCamera } from "./panCamera";
export function LockedPan({controlsRef,onAnchor}:{controlsRef:RefObject<OrbitHandle|null>;onAnchor:(anchor:PanAnchor|null)=>void}) {
  const {camera,gl,invalidate}=useThree();
  useEffect(()=>attachLockedPan(gl.domElement,()=>controlsRef.current,(dx,dy)=>{
    const controls=controlsRef.current;if(!controls)return;
    const bounds=gl.domElement.getBoundingClientRect();panCamera(camera,controls,dx,dy,bounds.width,bounds.height);invalidate();
  },onAnchor),[camera,gl,invalidate,controlsRef,onAnchor]);
  return null;
}
