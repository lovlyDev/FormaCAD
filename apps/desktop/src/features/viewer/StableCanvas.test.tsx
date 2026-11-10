import { render,cleanup } from "@testing-library/react";
import { afterEach,describe,it,expect,vi } from "vitest";
import { useEffect,type ReactNode } from "react";
const state=vi.hoisted(()=>({sources:[] as HTMLElement[],callbacks:[] as (()=>void)[]}));
vi.mock("@react-three/fiber",()=>({Canvas:({eventSource,children}:{eventSource:HTMLElement;children:ReactNode})=>{
 useEffect(()=>{state.sources.push(eventSource);state.callbacks.push(()=>eventSource.addEventListener("pointerdown",()=>{}, {once:true}));},[eventSource]);
 return <div>{children}</div>;
}}));
import { StableCanvas } from "./StableCanvas";
afterEach(()=>{cleanup();state.sources=[];state.callbacks=[];});
describe("asynchronous Canvas event target",()=>{
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
