# Plugins

[Documentation](index.md) · [Русский](../ru/plugins.md) · [Platform specification](plugin-platform.md) · [Author guide](plugin-authoring.md)

Plugins add parametric operations, commands, import and export formats, panels and settings to FormaCAD. A plugin is a WebAssembly component that runs in a Wasmtime sandbox inside the CAD app. It cannot see your files, network or programs: all it can do is call FormaCAD functions you granted permissions for.

## Installing

**From a file.** Settings → “Plugins” → “Install from file…” → pick a `.cadplugin`. FormaCAD checks the package and opens the install window. Use this for plugins outside the catalog and for authors' development builds. Neither the catalog nor the network is needed.

**From the catalog.** “Add to FormaCAD” on `plugins.formacad.ru` opens `formacad://plugins/install?id=…&version=…`. FormaCAD downloads the signed catalog entry and the package only from `plugins.formacad.ru`, verifies the signature and SHA-256 and shows the same install window. The catalog key is built into the app from version 1.4.0; a build without it answers “This build has no catalog key” for link installs — use a file then. To publish your own plugin in the catalog, see the [author guide](plugin-authoring.md#releasing-to-the-catalog).

**The install window** shows the name, author, version, source, checksum and permissions in plain words. Nothing is installed until you press “Install”. A `formacad://` link never runs anything by itself.

## Permissions

| Permission | Allows |
| --- | --- |
| `model.read` | Reading the current model: parameters, operations, bodies |
| `model.write` | Proposing model changes, applied as one undoable revision |
| `selection.read` | Seeing which bodies, faces or edges you selected |
| `file.import` | Reading the file you pick in the import dialog |
| `file.export` | Writing data to the file you pick in the save dialog |
| `storage` | Keeping its own data in the FormaCAD profile (up to 10 MB) |
| `network:<host>` | HTTPS requests to that host only |
| `notifications` | Showing notifications |
| `viewport.capture` | Record turntable videos of the model in the 3D view; the plugin never sees the model, the frames or the file |

A package that calls a function without the matching permission, or tries to reach the system outside FormaCAD (for example WASI), is not installed.

## Updates and versions

When you install a newer version, the window shows the installed version and marks **new** permissions as “New”. The old version stays active until you agree. Versions live side by side (`plugins/<id>/<version>/` in the profile); with several, the plugin card lets you pick the active one. Switching to a version with new permissions is not possible from settings — install it again to get the consent window. The same version with different contents is rejected.

## Using plugins

Actions of enabled plugins appear in the command palette (`Ctrl K`): “<plugin>: <operation>”, “…: import …”, “…: export …”, “…: panel …”. The same actions are available from the 3D-view toolbar: a feature or command whose author set an icon gets its own button with that icon (for example a video camera for video recording); the rest are in the “Plugin actions” menu (puzzle icon). The buttons show only while enabled plugins have actions; the menu closes on `Esc` or a click outside. An operation opens its parameter form (lengths in mm, angles in degrees) and then the usual model-change confirmation. FormaCAD checks the plugin result (CAD IR v2, IDs, BREP build) and saves **one** revision; on any error the model is unchanged. History labels the change “Plugin <name> <version>: <operation>”.

The result consists of ordinary CAD operations. The body stays visible, measurable and exportable even after the plugin is disabled or removed.

Plugin import opens the system file picker; the plugin receives only the bytes of the chosen file. Plugin export hands the current model to the plugin and writes the result where you choose.

## Turntable video recording

A plugin with the `viewport.capture` permission (for example [Turntable Video](https://github.com/lovlyDev/formacad-orbit-video)) can ask FormaCAD to record a video of the model turning in a circle. Steps: command palette or the command's button in the 3D-view toolbar → the plugin's command → form (resolution, frames per second, length and number of turns, direction, which axis points up, part tilt, camera height, zoom, background, MP4 or WebM, bitrate) → Run → the native “Save video” dialog. Such a command does not change the model, so no model-edit approval appears.

FormaCAD renders every frame of a copy of the model shown in the 3D view and encodes the video with the system's encoders (WebCodecs). Your view, camera and selection stay unchanged. The “Recording video” window shows the frame number and percentage; Stop or closing the window aborts the recording and deletes the unfinished file. Until the recording is complete the file is named `<name>.mp4.part`.

Limits: up to 3840 × 2160, up to 120 fps, up to 10 minutes and at most 36,000 frames, files up to 4 GB. If the system cannot encode the chosen format or size, FormaCAD says so — lower the resolution or pick the other format. Only the model is recorded: the grid, selection, dimensions and labels do not appear in the video.

## Settings, journal, removal

The plugin card in “Settings → Plugins”: version, author, the “Enabled” switch, permissions, plugin settings (when declared), the call journal and removal. The journal stays local in `plugins/journal.jsonl` in the profile and is never sent. “Remove” deletes all versions of the plugin and its data (`plugins/data/<id>`) after a second confirmation.

## Limits and errors

- Each plugin call is limited by a compute budget (fuel), 256 MB of memory and 10 s. Exceeding a limit stops only that call: FormaCAD keeps running and reports a plugin error.
- Plugin errors show the plugin ID and code, separately from CAD errors.
- Reopening a plugin operation with its previous parameters is not available yet: edit it as ordinary operations or run the plugin again.
- Plugin panels open as a dialog from the palette, not in the sidebar.
- If the CAD app is already running, a `formacad://` link on Windows opens a second window with the install dialog.
- Plugins work only in the desktop app.

## Verification

`cargo test --lib plugins::` (package, manifest, permissions, limits, end-to-end install of the example), `cargo test --features native-occt --test plugin_gear` (the example builds OCCT geometry), `npx vitest run src/features/plugins` (install window and forms), `npm run check:i18n`, `npm run check:docs`.
