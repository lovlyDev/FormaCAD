# Plugin author guide

[Documentation](index.md) · [Русский](../ru/plugin-authoring.md) · [Specification](plugin-platform.md) · [Plugins for users](plugins.md)

A FormaCAD plugin is a WebAssembly component implementing plugin-api 0.2 — [`formacad:plugin`](../plugin-api/wit/formacad-plugin.wit), packed as `.cadplugin`. A complete working example is [`plugins/examples/simple-gear`](../../plugins/examples/simple-gear/src/lib.rs): a gear built from ordinary CAD IR operations.

## Project layout

```text
my-plugin/
  Cargo.toml          crate-type = ["cdylib"], wit-bindgen dependency
  manifest.json       per docs/schemas/plugin-manifest.schema.json
  wit/world.wit       your world: include host + the exports you need
  src/lib.rs          export implementations
  locales/ru.json     strings for %key% manifest values and the plugin itself
  locales/en.json
  assets/…            optional data (read through assets.read)
```

`wit/world.wit` includes the whole host and exports only what you contribute:

```wit
package example:my-plugin;

world my-plugin {
    include formacad:plugin/host@0.1.0;
    export formacad:plugin/feature@0.1.0;
}
```

In `src/lib.rs`:

```rust
wit_bindgen::generate!({
    path: ["<path to docs/plugin-api/wit>", "wit"],
    world: "example:my-plugin/my-plugin",
    generate_all,
});
```

Keep a copy of `formacad-plugin.wit` in your repository (for example `wit/deps/formacad-plugin/`) and point to it. With the copy in `wit/deps/`, `path: "wit"` is enough: wit-bindgen picks up `deps` itself, and listing the folder again fails with “re-add package”. Only the host interfaces your code actually calls become component imports, and each must be covered by a permission in `manifest.json` (table in the [specification, section 11](plugin-platform.md#11-host-interface-formacadplugin-plugin-api-02)).

## What a plugin returns

- `feature.build` and `importer.import-file` return a JSON array of [Command API](command-api.md) commands: usually `add_feature` with CAD IR v2 operations (`circle`, `rectangle`, `sketch2d`, `extrude`, `revolve`, `rotate`, `translate`, `boolean`, `hole`, …) and `add_body`.
- `command.run` returns `{"message":"…","commands":[…]}`; an empty array means “change nothing”. A command without the `model.write` permission may only return an empty array.
- `exporter.export-file` returns the file bytes.
- `panel.render` returns `{"blocks":[{"type":"text","text":"…"},{"type":"form","fields":[…],"submit":{"command":"<id>","label":"…"}}]}`.
- Every new ID starts with `context.idPrefix`. Errors are `plugin-error { code, message }`; users see them as plugin errors.

The host validates form values before the call (type, min/max, options) and the result with strict CAD IR types, document validation and a BREP build. Any error leaves the model unchanged.

## Build and pack

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cargo build --release --target wasm32-unknown-unknown
```

```bash
cargo run --release --manifest-path <FormaCAD>/plugins/tools/cadplugin/Cargo.toml -- pack --manifest manifest.json --wasm target/wasm32-unknown-unknown/release/my_plugin.wasm --dir . --out dist
```

`cadplugin pack` turns the wit-bindgen core module into a component, adds `manifest.json`, `locales/`, `assets/` and the icon, and writes `dist/<id>-<version>.cadplugin` with fixed timestamps (same input → same SHA-256). Packages are limited to 20 MB packed, 60 MB unpacked, 200 files and path depth 6.

## Testing locally

Install `dist/*.cadplugin` via “Settings → Plugins → Install from file…”: FormaCAD runs the same checks as the catalog (manifest schema, imports against permissions, exports against `contributes`). Run the operation from the command palette (`Ctrl K`). The call journal is in the plugin card; `log.write` lines go there too.

Call limits: fuel, 256 MB of memory, 10 s. Keep computations deterministic and keep no in-memory state between calls — every call gets a fresh instance; use `storage` for data.

## Releasing to the catalog

1. The tag `vX.Y.Z` equals `version` in the manifest; `id` never changes after the first publication.
2. Attach `<id>-<version>.cadplugin` to the GitHub release.
3. Connect the repository once: sign in on `plugins.formacad.ru` with your FormaCAD account; link GitHub to the account on formacad.ru (“Account → Security”) if it is not linked yet; in the “Author dashboard” click “Install GitHub App”, install [FormaCAD Plugins](https://github.com/apps/formacad-plugins) on the repository (it only reads contents and metadata) and choose “Publish from here”.
4. From then on every published (not draft, not prerelease) GitHub release is checked automatically: asset, ZIP, manifest (`repository` = this repository, `author.github` = your GitHub), WASM imports against permissions, screenshots. A passing version appears in the catalog at once; the rejection reason is shown in the dashboard and “Check again” repeats the check. The catalog keeps its own copy of the package and signs the index entry.
5. Users re-confirm new permissions in a new version — declare only what you really need.

## Example: simple-gear

The example builds a disc and a rectangular tooth, rotates it N times, unions everything and drills a bore. Permissions: `model.write` (`features` contribution) and `notifications` (a message after building). The prebuilt package used by tests: [`plugins/examples/simple-gear/prebuilt`](../../plugins/examples/simple-gear/prebuilt). The [`plugin_gear.rs`](../../apps/desktop/src-tauri/tests/plugin_gear.rs) test compares the volume of the OCCT gear with an analytic formula.

## Example: recording a video (plugin-api 0.2)

A command without `model.write` can ask FormaCAD to record a turntable video of the model. Declare the `viewport.capture` permission, `"api": "0.2"` and `"minAppVersion": "1.5.0"`, check the form and call the host:

```rust
use formacad::plugin::viewport;
viewport::record_orbit(viewport::Orbit {
    width: 1920, height: 1080, fps: 30, duration_ms: 8000, turns: 1.0, clockwise: false,
    up: viewport::UpAxis::Z, tilt_deg: 0.0, elevation_deg: 20.0, start_deg: 0.0, zoom: 1.0,
    background: viewport::Background::Theme, format: viewport::VideoFormat::Mp4, bitrate_kbps: 0,
})?;
// then return {"message":"…","commands":[]}
```

Recording starts after the call ends: the CAD app asks for the path, renders the frames and encodes the video itself. A complete plugin with a form and translations: [lovlyDev/formacad-orbit-video](https://github.com/lovlyDev/formacad-orbit-video). Value ranges are in the [specification, section 12](plugin-platform.md#12-implementation-notes).
