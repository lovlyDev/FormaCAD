import { useRef } from "react";
import { StableCanvas } from "../viewer/StableCanvas";
import { OrbitControls } from "@react-three/drei";
import type { Group } from "three";
import type { OrbitHandle } from "../viewer/CameraControl";
import { PreviewFraming } from "./PreviewFraming";
import { useTheme } from "../../lib/theme";
import { t } from "../../i18n";
export function PreviewCanvas({object,fit}:{object:Group;fit:number}){
 const controls=useRef<OrbitHandle|null>(null),light=useTheme()==="light";
 return <div className="model-preview-canvas" role="img" aria-label={t("Draft model preview")}>
  <StableCanvas camera={{position:[120,100,120],fov:38,near:.1,far:10000}} dpr={[1,1.5]}>
   <color attach="background" args={[light?"#f5f7f9":"#202326"]}/><ambientLight intensity={1.6}/><directionalLight position={[80,160,100]} intensity={3}/>
   <primitive object={object} dispose={null}/><OrbitControls ref={controls as never} makeDefault panSpeed={.3} screenSpacePanning enableDamping={false} minDistance={.01} maxDistance={100000}/>
   <PreviewFraming object={object} fit={fit} controls={controls}/>
  </StableCanvas>
 </div>;
}
