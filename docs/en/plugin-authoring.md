# Plugin author guide

[Documentation](index.md) · [Русский](../ru/plugin-authoring.md) · [Specification](plugin-platform.md) · [Plugins for users](plugins.md)

A FormaCAD plugin is a WebAssembly component implementing [`formacad:plugin@0.1`](../plugin-api/wit/formacad-plugin.wit), packed as `.cadplugin`. A complete working example is [`plugins/examples/simple-gear`](../../plugins/examples/simple-gear/src/lib.rs): a gear built from ordinary CAD IR operations.

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

Keep a copy of `formacad-plugin.wit` in your repository (for example `wit/deps/formacad-plugin/`) and point to it. Only the host interfaces your code actually calls become component imports, and each must be covered by a permission in `manifest.json` (table in the [specification, section 11](plugin-platform.md#11-host-interface-formacadplugin01)).

## What a plugin returns

- `feature.build` and `importer.import-file` return a JSON array of [Command API](command-api.md) commands: usually `add_feature` with CAD IR v2 operations (`circle`, `rectangle`, `sketch2d`, `extrude`, `revolve`, `rotate`, `translate`, `boolean`, `hole`, …) and `add_body`.
- `command.run` returns `{"message":"…","commands":[…]}`; an empty array means “change nothing”.
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
3. Install the “FormaCAD Plugins” GitHub App on the repository and publish on `plugins.formacad.ru` (specification section 5). The catalog keeps its own copy of the package and signs the index entry.
4. Users re-confirm new permissions in a new version — declare only what you really need.

## Example: simple-gear

The example builds a disc and a rectangular tooth, rotates it N times, unions everything and drills a bore. Permissions: `model.write` (`features` contribution) and `notifications` (a message after building). The prebuilt package used by tests: [`plugins/examples/simple-gear/prebuilt`](../../plugins/examples/simple-gear/prebuilt). The [`plugin_gear.rs`](../../apps/desktop/src-tauri/tests/plugin_gear.rs) test compares the volume of the OCCT gear with an analytic formula.
