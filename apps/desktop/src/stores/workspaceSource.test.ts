import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "./workspace";
const mock=vi.hoisted(()=>({save:vi.fn(),close:vi.fn()}));
vi.mock("../lib/api",()=>({saveProject:mock.save}));
vi.mock("../features/project-sources/sourceApi",()=>({closeFolderSource:mock.close}));
const ref=(id="first",commitId="commit")=>({sourceId:id,commitId,taskScopeId:`group-${id}`});
const project=()=>({...newProject("Source","blank","mm","codex"),id:"shared",currentRevision:"head"});
beforeEach(()=>{
  mock.close.mockResolvedValue(true);useWorkspace.getState().setProject(null);
  localStorage.clear();mock.close.mockClear();mock.save.mockReset().mockImplementation(async p=>p);
});
it("clears topology/body selection when equal-ID source openings change",()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref());
  useWorkspace.getState().setSelected("body");useWorkspace.getState().setSelectedFace({bodyId:"body",faceOrdinal:2,revisionId:"head"});
  const first=useWorkspace.getState().sourceBinding;
  useWorkspace.getState().openFolderProject(p,ref("second"));
  expect(useWorkspace.getState().project).toBe(p);
  expect(useWorkspace.getState().sourceBinding?.openingId).not.toBe(first?.openingId);
  expect(useWorkspace.getState().selectedBodies).toEqual([]);expect(useWorkspace.getState().selectedFace).toBeNull();
  expect(mock.close).toHaveBeenCalledWith(ref());
});
it("a delayed managed save cannot replace an equal-ID folder view",async()=>{
  const p=project();useWorkspace.getState().setProject(p);
  let finish!:(value:typeof p)=>void;mock.save.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
  const pending=useWorkspace.getState().update({...p,name:"managed write"});
  useWorkspace.getState().openFolderProject(p,ref());const opening=useWorkspace.getState().sourceBinding;
  finish({...p,name:"late managed result"});await pending;
  expect(useWorkspace.getState().project).toBe(p);expect(useWorkspace.getState().sourceBinding).toBe(opening);
});
it("does not invoke managed persistence for a folder read reference",async()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref());
  await expect(useWorkspace.getState().update({...p,name:"attempt"})).rejects.toThrow("PROJECT_SOURCE_WRITE_NOT_AUTHORIZED");
  expect(mock.save).not.toHaveBeenCalled();expect(useWorkspace.getState().project).toBe(p);
});
it("refreshes folder metadata through the same family and rejects an older publication",()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref());useWorkspace.getState().setSelected("body");
  useWorkspace.getState().setSelectedFace({bodyId:"body",faceOrdinal:2,revisionId:"head"});
  const original=useWorkspace.getState().sourceBinding!,face=useWorkspace.getState().selectedFace;
  const metadata={...p,name:"metadata winner"};
  expect(useWorkspace.getState().publishFolderSnapshot(metadata,original,ref("first","new-commit"))).toBe(true);
  expect(useWorkspace.getState().sourceBinding?.openingId).toBe(original.openingId);
  expect(useWorkspace.getState().selectedFace).toBe(face);
  expect(useWorkspace.getState().publishFolderSnapshot(p,original,ref("first","older"))).toBe(false);
  expect(useWorkspace.getState().project).toBe(metadata);expect(mock.close).not.toHaveBeenCalled();
});
it("rejects a folder refresh that tries to change the original selector",()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref());const original=useWorkspace.getState().sourceBinding!;
  expect(()=>useWorkspace.getState().publishFolderSnapshot(p,original,ref("other"))).toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(useWorkspace.getState().sourceBinding).toBe(original);
});
it("keeps managed metadata stable but explicit equal-ID reopen starts a fresh family",()=>{
  const p=project();useWorkspace.getState().setProject(p);const original=useWorkspace.getState().sourceBinding!;
  useWorkspace.getState().setProject({...p,name:"metadata"});expect(useWorkspace.getState().sourceBinding).toBe(original);
  useWorkspace.getState().openManagedProject(p);expect(useWorkspace.getState().sourceBinding?.openingId).not.toBe(original.openingId);
});
it("does not read or overwrite managed selection preferences for a folder",()=>{
  const p=project(),key=`forma.ui.project.${p.id}.selected`;localStorage.setItem(key,JSON.stringify("managed-body"));
  useWorkspace.getState().openFolderProject(p,ref());expect(useWorkspace.getState().selected).toBeNull();
  useWorkspace.getState().setSelected("folder-body");
  expect(localStorage.getItem(key)).toBe(JSON.stringify("managed-body"));
  expect(JSON.stringify({...localStorage})).not.toContain(useWorkspace.getState().sourceBinding!.openingId);
});
it("releases each replaced folder selector once and never closes the replacement",()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref());useWorkspace.getState().openFolderProject(p,ref("second"));
  useWorkspace.getState().setProject(null);useWorkspace.getState().setProject(null);
  expect(mock.close.mock.calls.map(call=>call[0].sourceId)).toEqual(["first","second"]);
});
