import { act,renderHook } from "@testing-library/react";
import { afterEach,expect,it } from "vitest";
import { folderSource,managedSource,refreshFolderSource } from "./sourceBinding";
import {useSourcePreference} from "./useSourcePreference";
const ref=(id="first",commitId="commit")=>({sourceId:id,commitId,taskScopeId:`group-${id}`});
afterEach(()=>localStorage.clear());
it("keeps folder drafts in memory and out of stored managed values",()=>{
  const key="forma.ui.project.shared.prompt";localStorage.setItem(key,JSON.stringify("managed draft"));
  const first=folderSource("shared",ref()),second=folderSource("shared",ref("second"));
  const view=renderHook(({binding})=>useSourcePreference(key,"",binding),{initialProps:{binding:first}});
  expect(view.result.current[0]).toBe("");act(()=>view.result.current[1]("folder draft"));expect(view.result.current[0]).toBe("folder draft");
  const stale=view.result.current[1];view.rerender({binding:second});expect(view.result.current[0]).toBe("");
  act(()=>view.result.current[1]("second draft"));act(()=>stale("late first mutation"));
  expect(view.result.current[0]).toBe("second draft");expect(localStorage.getItem(key)).toBe(JSON.stringify("managed draft"));
  expect(JSON.stringify({...localStorage})).not.toContain(first.openingId);view.unmount();
});
it("preserves folder drafts across metadata refresh of the same opening",()=>{
  const before=Object.fromEntries(Object.keys(localStorage).map(key=>[key,localStorage.getItem(key)]));
  const first=folderSource("shared",ref());const view=renderHook(({binding})=>useSourcePreference("forma.ui.project.shared.prompt","",binding),{initialProps:{binding:first}});
  act(()=>view.result.current[1]("draft"));view.rerender({binding:refreshFolderSource(first,ref("first","metadata"))});
  expect(view.result.current[0]).toBe("draft");expect(Object.fromEntries(Object.keys(localStorage).map(key=>[key,localStorage.getItem(key)]))).toEqual(before);
  expect(localStorage.getItem("forma.ui.project.shared.prompt")).toBeNull();view.unmount();
});
it("ignores a setter captured before explicit managed reopen with the same ID",()=>{
  const first=managedSource("shared"),second=managedSource("shared");
  const view=renderHook(({binding})=>useSourcePreference("forma.ui.project.shared.prompt","",binding),{initialProps:{binding:first}});
  const stale=view.result.current[1];view.rerender({binding:second});act(()=>view.result.current[1]("new draft"));act(()=>stale("stale"));
  expect(view.result.current[0]).toBe("new draft");view.unmount();
});

it("resets a folder draft when its revision key changes within the same opening",()=>{
 const binding=folderSource("shared",ref());const view=renderHook(({key})=>useSourcePreference(key,"saved source",binding),{initialProps:{key:"revision-a"}});
 act(()=>view.result.current[1]("draft a"));const stale=view.result.current[1];view.rerender({key:"revision-b"});
 expect(view.result.current[0]).toBe("saved source");act(()=>view.result.current[1]("draft b"));act(()=>stale("late a"));
 expect(view.result.current[0]).toBe("draft b");expect(localStorage.getItem("revision-a")).toBeNull();expect(localStorage.getItem("revision-b")).toBeNull();view.unmount();
});
it("ignores a managed setter from a retired preference key",()=>{
 const binding=managedSource("shared");const view=renderHook(({key})=>useSourcePreference(key,"",binding),{initialProps:{key:"revision-a"}});
 const stale=view.result.current[1];view.rerender({key:"revision-b"});act(()=>view.result.current[1]("new draft"));act(()=>stale("late a"));
 expect(view.result.current[0]).toBe("new draft");expect(localStorage.getItem("revision-a")).toBeNull();view.unmount();
});
