import { afterEach,expect,it,vi } from "vitest";
import { act,cleanup,fireEvent,render,screen,waitFor } from "@testing-library/react";
import { PersistentDetails,useEditorPreference } from "./editorPreferences";
import { flushPreferences,restorePreferences } from "../../lib/persistence";
import { newProject,useWorkspace } from "../../stores/workspace";
const ipc=vi.hoisted(()=>({stored:{} as Record<string,string>,invoke:vi.fn()}));

const bridge=window as typeof window & {isTauri:boolean;__TAURI_INTERNALS__:{invoke:typeof ipc.invoke}};
function Preferences(){const [checked,setChecked]=useEditorPreference("preview.auto",false);return <PersistentDetails stateId="test"><summary>Test</summary><label><input type="checkbox" checked={checked} onChange={e=>setChecked(e.target.checked)}/>Auto</label></PersistentDetails>;}
afterEach(()=>{cleanup();bridge.isTauri=false;localStorage.clear();useWorkspace.setState({project:null});vi.clearAllMocks();});
it("flushes project-scoped section and checkbox state and restores it from the database boundary",async()=>{
 ipc.invoke.mockImplementation(async(command:string,args?:{value:Record<string,string>})=>{if(command==="set_ui_preferences")ipc.stored={...args!.value};return ipc.stored;});
 bridge.isTauri=true;bridge.__TAURI_INTERNALS__={invoke:ipc.invoke};
 const first=newProject("One","blank","mm","codex"),second=newProject("Two","blank","mm","codex");useWorkspace.setState({project:first});const mounted=render(<Preferences/>);
 const details=screen.getByText("Test").closest("details")!;act(()=>{details.open=true;fireEvent(details,new Event("toggle",{bubbles:true}));});fireEvent.click(screen.getByRole("checkbox"));await waitFor(()=>expect(localStorage.getItem(`forma.ui.project.${first.id}.modelEditor.sections.test`)).toBe("true"));await flushPreferences();
 expect(ipc.stored[`forma.ui.project.${first.id}.modelEditor.sections.test`]).toBe("true");expect(ipc.stored[`forma.ui.project.${first.id}.modelEditor.preview.auto`]).toBe("true");
 mounted.unmount();localStorage.clear();await restorePreferences();render(<Preferences/>);expect(screen.getByRole<HTMLInputElement>("checkbox").checked).toBe(true);expect(screen.getByText("Test").closest("details")!.open).toBe(true);
 act(()=>useWorkspace.setState({project:second}));expect(screen.getByRole<HTMLInputElement>("checkbox",{hidden:true}).checked).toBe(false);expect(screen.getByText("Test").closest("details")!.open).toBe(false);
});
