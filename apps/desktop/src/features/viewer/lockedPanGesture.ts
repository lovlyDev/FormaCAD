export type PanAnchor = { x: number; y: number };
export type PanControls = { enabled: boolean };
/** One RMB gesture owns its own lock; cleanup never exits another canvas's lock. */
export function attachLockedPan(canvas: HTMLCanvasElement, getControls: () => PanControls | null, pan: (dx:number,dy:number)=>void, onAnchor:(anchor:PanAnchor|null)=>void):()=>void {
  const document=canvas.ownerDocument, window=document.defaultView!;
  let mode:"idle"|"requesting"|"locked"|"fallback"="idle";
  let controls:PanControls|null=null, enabled=true, lastX=0,lastY=0,anchor:PanAnchor|null=null,disposed=false;
  const mark=()=>{canvas.dataset.panMode=mode;};
  const finish=()=>{
    mode="idle";mark();onAnchor(null);
    if(controls){controls.enabled=enabled;controls=null;}
    if(document.pointerLockElement===canvas)document.exitPointerLock();
  };
  const fallback=()=>{if(mode==="requesting"&&!disposed){mode="fallback";mark();onAnchor(null);}};
  const down=(event:PointerEvent)=>{
    if(event.button!==2||event.pointerType!=="mouse"||mode!=="idle")return;
    const next=getControls();if(!next||!next.enabled)return;
    event.preventDefault();controls=next;enabled=next.enabled;next.enabled=false;
    lastX=event.clientX;lastY=event.clientY;anchor={x:lastX,y:lastY};mode="requesting";mark();
    try {
      if(!canvas.requestPointerLock){fallback();return;}
      const result=canvas.requestPointerLock();
      if(result)void result.then(()=>{if(disposed&&document.pointerLockElement===canvas)document.exitPointerLock();},fallback);
    }catch{fallback();}
  };
  const lockChange=()=>{
    if(document.pointerLockElement===canvas){
      if(mode==="requesting"&&!disposed){mode="locked";mark();onAnchor(anchor);}
      else if(mode!=="locked")document.exitPointerLock();
    }else if(mode==="locked")finish();
  };
  const move=(event:MouseEvent)=>{
    if(mode==="idle"||mode==="requesting")return;
    if(!(event.buttons&2)){finish();return;}
    if(mode==="locked")pan(event.movementX,event.movementY);
    else {pan(event.clientX-lastX,event.clientY-lastY);lastX=event.clientX;lastY=event.clientY;}
  };
  const up=(event:MouseEvent)=>{if(event.button===2&&mode!=="idle")finish();};
  const key=(event:KeyboardEvent)=>{if(event.key==="Escape"&&mode!=="idle"){event.stopPropagation();finish();}};
  const hidden=()=>{if(document.hidden)finish();};
  const menu=(event:Event)=>event.preventDefault();
  mark();canvas.addEventListener("pointerdown",down,true);canvas.addEventListener("contextmenu",menu);
  document.addEventListener("mousemove",move);document.addEventListener("mouseup",up);document.addEventListener("pointerup",up);
  document.addEventListener("pointerlockchange",lockChange);document.addEventListener("pointerlockerror",fallback);
  document.addEventListener("keydown",key,true);document.addEventListener("visibilitychange",hidden);window.addEventListener("blur",finish);
  return ()=>{disposed=true;finish();canvas.removeEventListener("pointerdown",down,true);canvas.removeEventListener("contextmenu",menu);
    document.removeEventListener("mousemove",move);document.removeEventListener("mouseup",up);document.removeEventListener("pointerup",up);
    document.removeEventListener("pointerlockchange",lockChange);document.removeEventListener("pointerlockerror",fallback);
    document.removeEventListener("keydown",key,true);document.removeEventListener("visibilitychange",hidden);window.removeEventListener("blur",finish);};
}
