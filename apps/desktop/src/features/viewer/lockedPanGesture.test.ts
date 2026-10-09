import { afterEach,describe,expect,it,vi } from "vitest";
import { attachLockedPan } from "./lockedPanGesture";
let cleanup:()=>void=()=>{};let owner:Element|null=null;
function setup(){
 const canvas=document.createElement("canvas");document.body.append(canvas);owner=null;
 Object.defineProperty(document,"pointerLockElement",{configurable:true,get:()=>owner});
 const lock=()=>{owner=canvas;document.dispatchEvent(new Event("pointerlockchange"));};
 const exit=vi.fn(()=>{owner=null;document.dispatchEvent(new Event("pointerlockchange"));});Object.defineProperty(document,"exitPointerLock",{configurable:true,value:exit});
 const request=vi.fn(()=>Promise.resolve());Object.defineProperty(canvas,"requestPointerLock",{value:request});
 const controls={enabled:true},pan=vi.fn(),anchor=vi.fn();cleanup=attachLockedPan(canvas,()=>controls,pan,anchor);
 const down=()=>{const e=new MouseEvent("pointerdown",{button:2,clientX:30,clientY:40,bubbles:true,cancelable:true});Object.defineProperty(e,"pointerType",{value:"mouse"});canvas.dispatchEvent(e);};
 return {canvas,controls,pan,anchor,down,lock,exit,request};
}
afterEach(()=>{cleanup();document.body.innerHTML="";vi.restoreAllMocks();});
describe("locked pan lifecycle",()=>{
 it("uses relative deltas and releases on RMB up",()=>{
  const s=setup();s.down();expect(s.controls.enabled).toBe(false);s.lock();expect(s.anchor).toHaveBeenLastCalledWith({x:30,y:40});
  const e=new MouseEvent("mousemove",{buttons:2,clientX:30,clientY:40});Object.defineProperties(e,{movementX:{value:12},movementY:{value:-7}});document.dispatchEvent(e);expect(s.pan).toHaveBeenCalledWith(12,-7);
  document.dispatchEvent(new MouseEvent("mouseup",{button:2}));expect(s.controls.enabled).toBe(true);expect(owner).toBeNull();expect(s.anchor).toHaveBeenLastCalledWith(null);
  document.dispatchEvent(e);expect(s.pan).toHaveBeenCalledOnce();
 });
 it("releases a late lock after the gesture ended and leaves foreign locks alone",()=>{
  const s=setup();s.down();document.dispatchEvent(new MouseEvent("mouseup",{button:2}));s.lock();expect(owner).toBeNull();
  owner=document.createElement("canvas");cleanup();expect(owner).not.toBeNull();expect(s.controls.enabled).toBe(true);
 });
 it("falls back after rejection and exits on focus loss",async()=>{
  const s=setup();s.request.mockRejectedValueOnce(new Error("denied"));s.down();await Promise.resolve();expect(s.canvas.dataset.panMode).toBe("fallback");
  document.dispatchEvent(new MouseEvent("mousemove",{buttons:2,clientX:42,clientY:45}));expect(s.pan).toHaveBeenCalledWith(12,5);
  window.dispatchEvent(new Event("blur"));expect(s.controls.enabled).toBe(true);expect(s.canvas.dataset.panMode).toBe("idle");
 });
 it("Escape restores controls and marker",()=>{const s=setup();s.down();s.lock();document.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}));expect(owner).toBeNull();expect(s.controls.enabled).toBe(true);});
});
