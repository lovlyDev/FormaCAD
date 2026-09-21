import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import * as Tooltip from "@radix-ui/react-tooltip";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Updates } from "./Updates";

const mocks = vi.hoisted(() => ({invoke:vi.fn(), check:vi.fn(), relaunch:vi.fn(), download:vi.fn(), install:vi.fn(), close:vi.fn(), order:[] as string[]}));
afterEach(cleanup);
vi.mock("@tauri-apps/api/core", () => ({isTauri:()=>true,invoke:mocks.invoke}));
vi.mock("@tauri-apps/plugin-updater", () => ({check:mocks.check}));
vi.mock("@tauri-apps/plugin-process", () => ({relaunch:mocks.relaunch}));
function view(blocked=false) {return render(<Tooltip.Provider><Updates blocked={blocked}/></Tooltip.Provider>);}
beforeEach(() => {
  vi.clearAllMocks(); mocks.order.length=0;
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
