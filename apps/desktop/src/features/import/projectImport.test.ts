import { expect,it,vi } from "vitest";
import * as THREE from "three";
import { prepareProjectImport } from "./projectImport";
import { newProject } from "../../stores/workspace";
import { loadModel } from "../../lib/files";
vi.mock("../../lib/files",()=>({loadModel:vi.fn(async()=>new THREE.Group())}));
it("retains STEP as a conversion candidate without creating an empty geometry revision",async()=>{
 const original=newProject("Import","blank","mm","codex");const file={name:"Part.STP",kind:"model" as const,size:100,data:"data:application/step;base64,AA=="};
 const result=await prepareProjectImport(original,[file]);expect(result.step).toBe(file);expect(result.hasMesh).toBe(false);expect(result.project.files).toEqual([file]);expect(result.project.revisions).toHaveLength(0);expect(original.files).toHaveLength(0);
});
it("validates mesh before creating a revision and keeps source history untouched on failure",async()=>{
 const original=newProject("Import","blank","mm","codex");const file={name:"Part.stl",kind:"model" as const,size:100};
 const result=await prepareProjectImport(original,[file]);expect(result.project.revisions[0].source).toBe(file.name);expect(original.revisions).toHaveLength(0);
 vi.mocked(loadModel).mockRejectedValueOnce(new Error("invalid mesh"));await expect(prepareProjectImport(original,[file])).rejects.toThrow("invalid mesh");expect(original.files).toHaveLength(0);
 await expect(prepareProjectImport(result.project,[file])).rejects.toThrow();expect(result.project.revisions).toHaveLength(1);
});
