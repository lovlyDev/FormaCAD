import { afterEach, expect, it } from "vitest";
import { suppressBrowserContextMenu } from "./browserContextMenu";

let release = () => {};
afterEach(() => { release(); document.body.innerHTML = ""; });
const open = (element: Element) => { const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true }); element.dispatchEvent(event); return event.defaultPrevented; };

it("blocks the webview menu everywhere but text fields", () => {
  release = suppressBrowserContextMenu();
  document.body.innerHTML = `<div id="panel"><span>3D</span></div><input id="name"><textarea id="note"></textarea><div id="rich" contenteditable="true"><b>x</b></div>`;
  expect(open(document.querySelector("#panel span")!)).toBe(true);
  expect(open(document.querySelector("#name")!)).toBe(false);
  expect(open(document.querySelector("#note")!)).toBe(false);
  expect(open(document.querySelector("#rich b")!)).toBe(false);
});
