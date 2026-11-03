/**
 * The webview's own right-click menu (Back, Reload, Save as, Print, More tools) does not belong
 * in a desktop app. Text fields keep it for cut, copy and paste; app menus handle their own events.
 */
export function suppressBrowserContextMenu(target: Document = document) {
  const menu = (event: MouseEvent) => {
    const element = event.target instanceof Element ? event.target : null;
    if (element?.closest("input, textarea, [contenteditable=''], [contenteditable='true']")) return;
    event.preventDefault();
  };
  target.addEventListener("contextmenu", menu);
  return () => target.removeEventListener("contextmenu", menu);
}
