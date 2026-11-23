#!/usr/bin/env python3
"""Use the real WebView, native folder chooser and CAD worker on an isolated X display."""
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from pathlib import Path


DRIVER = "http://127.0.0.1:4444"


def request(method, path, payload=None, timeout=100):
    body = None if payload is None else json.dumps(payload).encode()
    query = urllib.request.Request(
        DRIVER + path, data=body, method=method, headers={"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(query, timeout=timeout) as response:
            return json.load(response)["value"]
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"{path}: {error.read().decode()}") from error


def wait_for(check, description, timeout=25):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for {description}")


def driver_ready():
    try:
        return request("GET", "/status", timeout=1).get("ready")
    except (urllib.error.URLError, ConnectionError, TimeoutError, RuntimeError):
        # The Tauri driver can accept requests before its native driver is listening.
        return False


@contextmanager
def application(binary):
    session = request("POST", "/session", {
        "capabilities": {"alwaysMatch": {"tauri:options": {"application": str(binary)}}},
    })["sessionId"]
    route = f"/session/{session}"
    try:
        request("POST", route + "/timeouts", {"script": 90000})
        wait_for(
            lambda: request("POST", route + "/execute/sync", {
                "script": "return Boolean(window.__TAURI_INTERNALS__ && document.querySelector('.topbar'));",
                "args": [],
            }),
            "Forma WebView",
        )
        yield route
    finally:
        request("DELETE", route)


def invoke(route, command, args):
    result = request("POST", route + "/execute/async", {
        "script": """
           const [command, args, done] = arguments;
           window.__TAURI_INTERNALS__.invoke(command, args).then(
              value => done({ ok: true, value }),
              error => done({ ok: false, error: String(error) }),
           );
        """,
        "args": [command, args],
    })
    if not result["ok"]:
        raise AssertionError(f"{command}: {result['error']}")
    return result.get("value")


def visible_picker():
    result = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--name", "^Forma CAD$"],
        text=True, capture_output=True, check=False,
    )
    found = result.stdout.split()
    if len(found) > 1:
        raise AssertionError(f"More than one folder picker visible: {found}")
    return found[0] if found else None


def picker_control(name):
    import pyatspi
    from gi.repository import GLib

    def find(node):
        pending = deque([node])
        while pending:
            node = pending.popleft()
            if node is None:
                continue
            try:
                if (node.name.lstrip("_") == name
                        and node.getState().contains(pyatspi.STATE_SHOWING)):
                    return node
                pending.extend(node)
            except GLib.Error as error:
                if "does not exist" not in str(error):
                    raise
                # GTK replaces completion rows when the location entry changes.
        return None

    for application in pyatspi.Registry.getDesktop(0):
        for dialog in application:
            if dialog is not None and dialog.name == "Forma CAD":
                return find(dialog)
    return None


def click_picker_control(name):
    import pyatspi

    control = wait_for(lambda: picker_control(name), f"GTK control {name!r}")
    x, y, width, height = control.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
    assert width > 0 and height > 0, f"GTK control {name!r} has no clickable area"
    subprocess.run(
        ["xdotool", "mousemove", str(x + width // 2), str(y + height // 2), "click", "1"],
        check=True,
    )


def save_picker_screenshot():
    import gi

    gi.require_version("Gdk", "3.0")
    from gi.repository import Gdk

    display = Gdk.Display.open(os.environ["DISPLAY"])
    root = display.get_default_screen().get_root_window()
    view = Gdk.pixbuf_get_from_window(root, 0, 0, root.get_width(), root.get_height())
    view.savev("storage-native-evidence/native-picker-failure.png", "png", [], [])


def select_folder(route, command, args, path):
    with ThreadPoolExecutor(max_workers=1) as executor:
        response = executor.submit(invoke, route, command, args)
        picker = wait_for(visible_picker, "native folder chooser")
        pid = subprocess.check_output(["xdotool", "getwindowpid", picker], text=True).strip()
        try:
            subprocess.run(["xdotool", "windowfocus", "--sync", picker], check=True)
            click_picker_control("Storage fixture")
            click_picker_control(path.name)
            click_picker_control("Open")
            wait_for(lambda: visible_picker() != picker, "GTK folder chooser to close")
        except Exception:
            try:
                save_picker_screenshot()
            except Exception as error:
                print(f"Could not capture GTK chooser: {error}", file=sys.stderr)
            raise
        return pid, response.result(timeout=30)


def write(route, opening, command, intent):
    project_id = opening["project"]["id"]
    source_id = opening["sourceId"]
    commit_id = opening["commitId"]
    context = {"projectId": project_id, "sourceId": source_id, "expectedCommitId": commit_id}
    approval = invoke(route, "request_permission", {
        **context, "action": "modify_project", "detail": "Storage native desktop fixture",
        "folderIntent": intent,
    })
    if approval["requiresConfirmation"]:
        invoke(route, "resolve_permission", {**context, "id": approval["id"], "allow": True})
    result = invoke(route, command, {"projectId": project_id, "folderWrite": {
        "sourceId": source_id, "expectedCommitId": commit_id, "approvalId": approval["id"],
    }})
    assert result["status"] == "committed", result
    assert (result["sourceId"], result["projectId"], result["baseCommitId"]) == (
        source_id, project_id, commit_id,
    )
    assert result["commitId"] != commit_id
    return result


def program(width):
    return json.dumps({
        "schemaVersion": 2, "revisionId": "desktop_fixture", "parameters": [],
        "features": [
            {"id": "profile", "name": "Profile", "operation": {"type": "rectangle",
             "width": {"kind": "literal", "mm": width}, "depth": {"kind": "literal", "mm": 20}}},
            {"id": "pad", "name": "Pad", "operation": {"type": "extrude", "sketchId": "profile",
             "distance": {"kind": "literal", "mm": 10}}},
        ],
        "bodies": [{"id": "body", "name": "Body", "sourceFeatureId": "pad"}],
    })


def verify(binary, folder):
    with application(binary) as route:
        original_pid, creation = select_folder(route, "create_project_folder", {
            "draft": {"name": "Native storage fixture", "units": "mm", "agent": "codex"},
        }, folder)
        assert creation["status"] == "opened", creation
        opening = creation["source"]
        assert opening["project"]["currentRevision"] is None
        first = write(route, opening, "apply_program", {
            "kind": "model", "program": program(40), "prompt": "Native storage test", "expectedRevision": None,
        })
        first_revision = first["project"]["currentRevision"]
        assert first_revision
        saved = write(route, first, "apply_program", {
            "kind": "model", "program": program(80), "prompt": "Native storage test",
            "expectedRevision": first_revision,
        })
        revision = saved["project"]["currentRevision"]
        assert revision and saved["project"]["revisions"][-1]["source"].endswith(".step")
        assert invoke(route, "close_project_source", {"sourceId": opening["sourceId"]})

    assert (folder / "project.sqlite").is_file()
    with application(binary) as route:
        reopened_pid, reopened = select_folder(route, "open_project_folder", {}, folder)
        assert original_pid != reopened_pid, "Expected an independent process after closing the first app"
        assert reopened["project"]["id"] == saved["project"]["id"]
        assert reopened["commitId"] == saved["commitId"]
        assert reopened["sourceId"] != opening["sourceId"]
        assert reopened["project"]["currentRevision"] == revision
        status = invoke(route, "history_status", {
            "projectId": reopened["project"]["id"], "sourceId": reopened["sourceId"],
            "expectedCommitId": reopened["commitId"],
        })
        assert status["canUndo"] and status["expectedRevision"] == revision
        undone = write(route, reopened, "undo_model", {
            "kind": "history", "direction": "undo", "expectedRevision": revision,
        })
        assert undone["project"]["currentRevision"] == first_revision
        redone = write(route, undone, "redo_model", {
            "kind": "history", "direction": "redo", "expectedRevision": first_revision,
        })
        assert redone["project"]["currentRevision"] == revision


def main():
    binary = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="forma-storage-desktop-") as directory:
        root = Path(directory)
        folder = root / "selected-FCP1"
        folder.mkdir()
        config = root / "config"
        gtk = config / "gtk-3.0"
        gtk.mkdir(parents=True)
        (gtk / "bookmarks").write_text(f"{root.as_uri()} Storage fixture\n", encoding="utf-8")
        env = dict(os.environ, FORMA_DATA_DIR=str(root / "app"), XDG_CONFIG_HOME=str(config))
        with subprocess.Popen(["tauri-driver"], env=env) as driver:
            try:
                wait_for(driver_ready, "tauri-driver")
                verify(binary, folder)
            finally:
                if driver.poll() is None:
                    driver.terminate()
                driver.wait(timeout=10)
    print("FORMA_NATIVE_STORAGE_DESKTOP_OK")


if __name__ == "__main__":
    main()
