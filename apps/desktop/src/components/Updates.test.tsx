import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import * as Tooltip from "@radix-ui/react-tooltip";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Updates } from "./Updates";

const mocks = vi.hoisted(() => ({invoke:vi.fn(), check:vi.fn(), relaunch:vi.fn(), openUrl:vi.fn(), download:vi.fn(), install:vi.fn(), close:vi.fn(), order:[] as string[]}));
afterEach(cleanup);
vi.mock("@tauri-apps/api/core", () => ({isTauri:()=>true,invoke:mocks.invoke}));
vi.mock("@tauri-apps/plugin-updater", () => ({check:mocks.check}));
vi.mock("@tauri-apps/plugin-process", () => ({relaunch:mocks.relaunch}));
vi.mock("@tauri-apps/plugin-opener", () => ({openUrl:mocks.openUrl}));
function view(blocked=false) {return render(<Tooltip.Provider><Updates blocked={blocked}/></Tooltip.Provider>);}
beforeEach(() => {
  vi.clearAllMocks(); mocks.order.length=0;
  mocks.openUrl.mockResolvedValue(undefined);
  mocks.invoke.mockImplementation(async (cmd:string) => {
    mocks.order.push(cmd);
    if(cmd === "update_support") return {configured:true,supported:true};
  });
  mocks.check.mockResolvedValue({version:"1.0.1",body:"Release notes",download:mocks.download,install:mocks.install,close:mocks.close});
  mocks.download.mockImplementation(async()=>{mocks.order.push("download");});
  mocks.install.mockImplementation(async()=>{mocks.order.push("install");});
  mocks.relaunch.mockImplementation(async()=>{mocks.order.push("relaunch");});
});
async function open() {
  fireEvent.click(screen.getByRole("button",{name:"Updates"}));
  await screen.findByText("Forma 1.0.1 is available");
}
describe("signed update interaction",()=>{
  it("renders release Markdown and resolves documentation links safely",async()=>{
    mocks.check.mockResolvedValueOnce({version:"1.2.1",body:"# Changes\n\nA **clearer** dialog.\n\n- First item\n- Second item\n\n[Guide](../en/data.md) [Unsafe](javascript:alert(1))",download:mocks.download,install:mocks.install,close:mocks.close});
    view(); fireEvent.click(screen.getByRole("button",{name:"Updates"}));
    expect(await screen.findByRole("heading",{name:"Changes"})).toBeVisible();
    expect(screen.getByText("clearer").tagName).toBe("STRONG");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(screen.getByRole("link",{name:"Guide"})).toHaveAttribute("href","https://github.com/lovlyDev/FormaCAD/blob/v1.2.1/docs/en/data.md");
    expect(screen.getByRole("link",{name:"Guide"})).toHaveAttribute("target","_blank");
    fireEvent.click(screen.getByRole("link",{name:"Guide"}));
    expect(mocks.openUrl).toHaveBeenCalledWith("https://github.com/lovlyDev/FormaCAD/blob/v1.2.1/docs/en/data.md");
    expect(screen.queryByRole("link",{name:"Unsafe"})).not.toBeInTheDocument();
  });
  it("Later does not download or install",async()=>{
    view(); await open();
    fireEvent.click(screen.getByRole("button",{name:"Later"}));
    expect(mocks.download).not.toHaveBeenCalled();
    expect(mocks.install).not.toHaveBeenCalled();
  });
  it("saves preferences and backs up before installation and restart",async()=>{
    localStorage.setItem("forma.ui.project.example.prompt","\"draft\"");
    view(); await open(); fireEvent.click(screen.getByRole("button",{name:"Install"}));
    await waitFor(()=>expect(mocks.relaunch).toHaveBeenCalled());
    expect(mocks.order.filter(command => ["download","prepare_update","install","relaunch"].includes(command))).toEqual(["download","prepare_update","install","relaunch"]);
    expect(mocks.invoke).toHaveBeenCalledWith("prepare_update",expect.objectContaining({preferences:expect.objectContaining({"forma.ui.project.example.prompt":"\"draft\""})}));
  });
  it("shows download progress and removes the bar during installation",async()=>{
    let emit!: (event: unknown) => void;
    let finish!: () => void;
    mocks.download.mockImplementationOnce((onEvent: (event: unknown) => void) => new Promise<void>(resolve => {
      emit = onEvent;
      finish = resolve;
    }));
    view(); await open(); fireEvent.click(screen.getByRole("button",{name:"Install"}));
    const bar = await screen.findByRole("progressbar",{name:"Downloading update"});
    expect(bar).not.toHaveAttribute("aria-valuenow");
    expect(bar).toHaveClass("indeterminate");
    await act(async()=>emit({event:"Started",data:{contentLength:1000}}));
    expect(bar).toHaveAttribute("aria-valuenow","0");
    await act(async()=>emit({event:"Progress",data:{chunkLength:250}}));
    expect(bar).toHaveAttribute("aria-valuenow","25");
    expect(screen.getByText("25%")).toBeVisible();
    expect(bar.querySelector(".update-progress-fill")).toHaveStyle({width:"25%"});
    await act(async()=>finish());
    await waitFor(()=>expect(mocks.install).toHaveBeenCalled());
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });
  it("a failed download never starts backup, installation or restart",async()=>{
    mocks.download.mockRejectedValueOnce(new Error("signature verification failed"));
    view(); await open(); fireEvent.click(screen.getByRole("button",{name:"Install"}));
    await screen.findByText(/The update could not be completed/);
    expect(mocks.install).not.toHaveBeenCalled();
    expect(mocks.relaunch).not.toHaveBeenCalled();
  });
  it("does not install while an operation is active",async()=>{
    view(true);await open();expect(screen.getByRole("button",{name:"Install"})).toBeDisabled();
  });
  it("automatically proposes a newer version at startup",async()=>{
    vi.useFakeTimers();
    try {view(); await act(async()=>{await vi.advanceTimersByTimeAsync(1600);});expect(screen.getByRole("dialog")).toBeVisible();expect(mocks.download).not.toHaveBeenCalled();}
    finally {vi.useRealTimers();}
  });
});
