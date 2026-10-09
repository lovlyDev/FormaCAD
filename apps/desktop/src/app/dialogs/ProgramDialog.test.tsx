import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { ProgramDialog } from "./ProgramDialog";
import { t } from "../../i18n";
import { useWorkspace, newProject } from "../../stores/workspace";
vi.mock("../../lib/api",async importOriginal=>({...await importOriginal<typeof import("../../lib/api")>(),native:true}));
vi.mock("../../features/model-preview/useModelPreview",()=>({useModelPreview:()=>({available:false,pending:false,waiting:false,preview:null,error:null})}));
afterEach(()=>{cleanup();useWorkspace.setState({project:null});localStorage.clear();});
it("an imported model cannot be replaced by a starter sketch or a persisted unrelated draft",()=>{
 const project=newProject("Imported","blank","mm","codex");useWorkspace.setState({project});
 localStorage.setItem(`forma.ui.project.${project.id}.source.new`,JSON.stringify("unrelated saved draft"));
 const apply=vi.fn();render(<ProgramDialog open close={()=>{}} source="" importedSource="part.stl" disabled={false} nativeCadAvailable nativeCadChecked onApply={apply}/>);
 expect(screen.getByText("part.stl")).toBeTruthy();expect(screen.queryByRole("button",{name:t("Create 2D sketch")})).toBeNull();
 expect(screen.getByRole("button",{name:t("Построить")}).hasAttribute("disabled")).toBe(true);
 expect(screen.queryByRole("textbox",{name:t("CAD source")})).toBeNull();expect(apply).not.toHaveBeenCalled();
});
