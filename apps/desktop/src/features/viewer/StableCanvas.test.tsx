import { render,cleanup,fireEvent } from "@testing-library/react";
import { afterEach,describe,it,expect,vi } from "vitest";
import { useEffect,type ReactNode,type CSSProperties } from "react";
const state=vi.hoisted(()=>({sources:[] as HTMLElement[],callbacks:[] as (()=>void)[]}));
vi.mock("@react-three/fiber",()=>({Canvas:({eventSource,children,style}:{eventSource:HTMLElement;children:ReactNode;style?:CSSProperties})=>{
 useEffect(()=>{state.sources.push(eventSource);state.callbacks.push(()=>eventSource.addEventListener("pointerdown",()=>{}, {once:true}));},[eventSource]);
 return <div data-testid="canvas-container" style={{pointerEvents:eventSource?"none":"auto",...style}}><canvas/>{children}</div>;
}}));
import { StableCanvas } from "./StableCanvas";
afterEach(()=>{cleanup();state.sources=[];state.callbacks=[];});
describe("asynchronous Canvas event target",()=>{
 it("allows the canvas to be hit while events bubble to its stable ancestor",()=>{
  const view=render(<StableCanvas/>);
  const container=view.getByTestId("canvas-container");
  expect(container.style.pointerEvents).toBe("auto");
  const listener=vi.fn();
  state.sources[0].addEventListener("pointerdown",listener);
  fireEvent.pointerDown(container.querySelector("canvas")!);
  expect(listener).toHaveBeenCalledOnce();
 });
 it("preserves caller layout styles while enabling canvas hit testing",()=>{
  const view=render(<StableCanvas style={{height:240,background:"black"}}/>);
  const container=view.getByTestId("canvas-container");
  expect(container.style.height).toBe("240px");
  expect(container.style.background).toBe("black");
  expect(container.style.pointerEvents).toBe("auto");
 });
 it("provides a concrete connected node before starting Canvas",()=>{
  render(<StableCanvas><span>scene</span></StableCanvas>);
  expect(state.sources).toHaveLength(1);
  expect(state.sources[0]).toBeInstanceOf(HTMLElement);
  expect(state.sources[0].isConnected).toBe(true);
 });
 it("keeps the target valid when renderer creation finishes after unmount",()=>{
  const view=render(<StableCanvas/>);
  const source=state.sources[0];
  view.unmount();
  expect(source.isConnected).toBe(false);
  expect(()=>state.callbacks[0]()).not.toThrow();
  expect(source).toBeInstanceOf(HTMLElement);
 });
});
