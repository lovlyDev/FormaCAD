# FormaCAD plugin platform

[Documentation](index.md) · [Русский](../ru/plugin-platform.md)

Status: **the CAD plugin runtime is implemented** (install from file, execution, permissions, UI — see [Plugins](plugins.md) and the [author guide](plugin-authoring.md)); the `plugins.formacad.ru` catalog is live (publishing through the “FormaCAD Plugins” GitHub App) and its signing key is built into the CAD app. This document is the shared contract for three parts built in parallel: the plugin runtime in the CAD app, the `plugins.formacad.ru` catalog and the author tooling. The catalog opens to everyone only after the CAD app can install and run plugins. Contract version: `plugin-api 0.1`; fields may change until the first public version and only compatibly afterwards (see “Versions and compatibility”).

Builds on the [2.x plan](../architecture-2.0.md), section 8.3 “Lua and WASM extension lifecycle” and stages 2.12–2.13.

## 1. What a plugin is

A plugin is a WebAssembly module with a manifest, packaged as a `.cadplugin` archive. It runs inside the CAD app in a **Wasmtime** sandbox and has no direct access to files, network, processes or the OS. Everything a plugin can do goes through host functions, each requiring a permission declared in the manifest and approved by the user.

A plugin can contribute (manifest field `contributes`):

| Contribution | What the user gets | How it applies |
| --- | --- | --- |
| `features` | New parametric operations: gears, threads, profiles… | The plugin turns parameters into CAD IR operations; the kernel builds geometry. The document stores plugin ID, pinned version, parameters and the last valid geometry |
| `commands` | Commands and actions in menus and the palette | A command produces a change plan through the Command API, applied as one transaction with rollback |
| `importers` / `exporters` | New file formats | Receives/returns bytes of the file the user picked; import results are CAD IR |
| `panels` / `settings` | Panels and settings | A declarative UI schema rendered by the CAD app with its own components, in both themes and accessibly. Arbitrary HTML/JS/CSS is not allowed |

A plugin cannot read arbitrary paths, start programs, open sockets, change the model bypassing the Command API, render its own HTML or keep running after the project closes.

## 2. The `.cadplugin` package

A ZIP archive; limits are checked before extraction: at most 20 MB compressed and 60 MB extracted, at most 200 files, path depth ≤ 6, relative paths only, no `..`, no symlinks.

```text
manifest.json        required, UTF-8
plugin.wasm          required, entry point (name set by manifest.entry)
icon.png             optional, 256×256, unless a built-in icon is used
locales/ru.json      optional, plugin string translations
locales/en.json
assets/…             optional, data the plugin reads through the host API
```

Screenshots and README are **not** packaged: the catalog takes them from the repository (section 6).

## 3. `manifest.json`

Formal schema: [`docs/schemas/plugin-manifest.schema.json`](../schemas/plugin-manifest.schema.json); example: [`docs/schemas/plugin-manifest.example.json`](../schemas/plugin-manifest.example.json).

| Field | Req. | Description |
| --- | --- | --- |
| `id` | yes | Unique ID: `^[a-z0-9]+(?:-[a-z0-9]+)*$`, 3–64 characters. Reserved for the author on first publication and never changes |
| `name` | yes | Name, up to 50 characters |
| `version` | yes | SemVer `MAJOR.MINOR.PATCH`; must equal the release tag `vX.Y.Z` |
| `description` | yes | One line up to 140 characters — catalog subtitle |
| `about` | no | Longer description up to 1000 characters — the About block |
| `highlights` | no | Up to 4 chips under About: `{ "icon": "<lucide name>", "label": "…" }` |
| `categories` | yes | 1–3 values from the fixed list (section 7) |
| `author` | yes | `{ "name": "…", "github": "<login>" }`; `github` must match the owner of the GitHub App installation (section 5) |
| `repository` | yes | `https://github.com/<owner>/<repo>` |
| `license` | yes | SPDX identifier (`MIT`, `Apache-2.0`, …) |
| `icon` | no | Built-in `lucide:<name>` or a PNG path in the package; chip background `iconColor` (`#rrggbb`) |
| `screenshots` | no | Up to 8: `{ "src": "<repository path>", "caption": "…", "theme": "light" \| "dark" \| "any" }`; PNG/WebP/JPEG, up to 2 MB, 800–2400 px wide |
| `minAppVersion` | yes | Minimum FormaCAD version (`2.12.0`) |
| `api` | yes | Plugin contract version (`0.1` or `0.2`) |
| `entry` | yes | WASM file name in the package |
| `permissions` | yes | Permission list (section 4); may be empty |
| `contributes` | yes | What the plugin adds (section 1); at least one contribution |
| `links` | no | `{ "homepage", "funding", "issues" }` — `https://` only |
| `locales` | no | Languages translated in the package |

Manifest strings are author content and are not translated by the CAD app. Plugin translations live in `locales/*.json` and are referenced as `%key%`.

## 4. Permissions

Permissions are declared in the manifest, shown at install time and take effect only after approval. A plugin cannot call a host function without its permission; this is checked when the module loads: importing a host function without the declared permission rejects installation.

| Permission | Grants |
| --- | --- |
| `model.read` | Read the current model: parameters, operations, bodies, measurements |
| `model.write` | Propose changes through the Command API (applied as a transaction, visible in history) |
| `selection.read` | Read the current face/edge/body selection |
| `file.import` | Receive bytes of a file the user picked in the import dialog |
| `file.export` | Return bytes for a file whose path the user picked |
| `storage` | Own storage up to 10 MB inside the CAD profile |
| `network:<host>` | HTTPS requests to the named host only; each host is a separate permission |
| `notifications` | Show CAD notifications |
| `viewport.capture` | Record turntable videos of the model in the 3D view (since plugin-api 0.2); the plugin never sees the geometry, the frames or the file path |

If a new version requests **new** permissions, the CAD app never updates silently: it shows the difference and waits for approval, keeping the previous version until the user agrees.

## 5. Publishing: GitHub only

The author's GitHub repository is the source of truth. Manual uploads to the site are not possible.

1. The author signs in to `plugins.formacad.ru` with any FormaCAD sign-in method; publishing requires a linked GitHub account.
2. The author installs the **FormaCAD Plugins** GitHub App on the repository. The installation proves repository access (also for organisations) and subscribes us to release events. Matching usernames is not sufficient and is not used.
3. The author clicks “Publish” and picks the repository. The server checks that the App installation belongs to the author's linked GitHub account and that `manifest.author.github` matches it.
4. The author creates a release tagged `vX.Y.Z` with `<id>-<version>.cadplugin` attached.
5. On `release.published` (fallback: hourly polling) the server:
   - downloads the package and checks archive limits, the manifest schema, `version == tag`, and that `id` belongs to this author;
   - validates the WASM: a valid module importing only `formacad:plugin@0.1` host functions, each import covered by a declared permission, no WASI filesystem/network access;
   - computes SHA-256, **keeps its own copy** of the package and copies screenshots (GitHub is unreliable from Russia; users download only from our server);
   - fetches the README and sanitises the Markdown (no raw HTML, images only through our proxy);
   - signs the index entry with the catalog key (Ed25519, separate from token and entitlement keys) and **publishes automatically**.
6. A version that fails validation is not published; the author sees the reason in their dashboard.

Moderation is automatic (owner decision). Protection against hijacked author accounts: new permissions require fresh consent in the CAD app; an administrator can **yank** a version or delist a plugin — the CAD app warns users who installed it and never installs a yanked version; a “Report” button on every plugin page.

## 6. The `plugins.formacad.ru` catalog

Modelled on the Obsidian community catalog, in FormaCAD style, RU/EN, both themes.

- **Home:** search, Popular / New / Updated, categories with counts.
- **Plugin page:** icon, name, author with GitHub avatar (links to the profile), install count; “Add to FormaCAD” and a “…” menu (GitHub, report a bug, request a feature, report the plugin); screenshot carousel (for the current theme when `theme` is set); tabs Overview (README collapsed with a smooth “Show more”), Checks, Updates (release list with notes).
- **Sidebar:** Health (from GitHub: last commit, release cadence, issue responsiveness — honest metrics, no invented scores), Checks (manifest, WASM, hash, requested permissions), About + `highlights` chips, Details (version, updated, created, releases, installs, works with FormaCAD X+, license), Permissions, Author.
- **Author profile:** avatar, name, login and bio from GitHub, social links (GitHub `social_accounts` plus links added on the site, `https://` only), their plugins.

Sign-in on `plugins.formacad.ru` goes through the `formacad.ru` account server with OAuth 2.0 + PKCE (the catalog is a separate client, like the CAD app). No separate passwords or cross-domain cookies.

## 7. Categories

Fixed list (translated by the CAD app and the catalog): `modeling`, `sketching`, `import-export`, `measurement`, `manufacturing`, `automation`, `ai`, `ui`, `library`, `other`.

## 8. Installing in the CAD app

1. “Add to FormaCAD” opens `formacad://plugins/install?id=<id>&version=<version>`.
2. The CAD app fetches the signed index entry from `https://plugins.formacad.ru/api/v1/plugins/<id>/versions/<version>`, verifies the signature with the catalog key built into the app, downloads the package **from our server** and checks its SHA-256.
3. It shows a dialog: name, author, version, permissions in plain words and a link to the page. Nothing installs without clicking “Install”.
4. The package unpacks into the CAD profile (`plugins/<id>/<version>/`); several versions may coexist; a project pins the version it uses.
5. Installed plugins work offline and without the catalog. A `formacad://` link never executes anything without confirmation.

## 9. Execution

Wasmtime with limits: fuel per call, up to 256 MB memory per instance, up to 10 s per call (adjustable for operations), interruption without closing the CAD app. A plugin error rolls back its transaction and is reported as a plugin error, not a CAD error. Call logs stay local.

If a plugin is missing or disabled, a body created by its operation stays visible, measurable and exportable from the last valid geometry, but the operation and dependent rebuilds are blocked until the required version is installed.

## 10. Versions and compatibility

`api` is the contract MAJOR.MINOR. MINOR adds host functions and fields compatibly; MAJOR breaks. The CAD app supports the current MAJOR and announces deprecations one release ahead. A plugin operation's parameter schema is versioned with the plugin; when a project updates, parameters migrate after a backup and a successful rebuild.

## 11. Host interface `formacad:plugin` (plugin-api 0.2)

WIT file: [`docs/plugin-api/wit/formacad-plugin.wit`](../plugin-api/wit/formacad-plugin.wit). A plugin is a **WebAssembly component** (component model), not a core module: WIT describes the types exactly, every group of host functions is its own interface, and permissions are checked against the component's import list without guessing an ABI. A component imports only the interfaces it actually calls.

| Interface | Permission | Functions |
| --- | --- | --- |
| `types`, `log`, `assets` | none | error types; local journal lines; reading the plugin's own package files (`assets/`, `locales/`) |
| `model` | `model.read` | `document()` — CAD IR v2 of the current revision (JSON) |
| `selection` | `selection.read` | `current()` — viewport selection (JSON) |
| `storage` | `storage` | `get/set/remove/keys`, 10 MB in total |
| `notifications` | `notifications` | `notify(level, message)`, up to 5 per call |
| `network` | `network:<host>` | `fetch(request)` — HTTPS to declared hosts only, no redirects, 15 s, response up to 10 MB |
| `viewport` | `viewport.capture` | `record-orbit(orbit)` — queue a turntable video of the model in the 3D view (since plugin-api 0.2) |

Plugin exports and the permissions they require:

| Export | Contribution | Permission | Result |
| --- | --- | --- | --- |
| `feature.build(id, params, context)` | `features` | `model.write` | JSON array of Command API commands |
| `command.run(id, params, context)` | `commands` | `model.write` when it changes the model | `{"message","commands":[…]}` |
| `importer.import-file(id, name, bytes, params, context)` | `importers` | `file.import` | JSON array of commands |
| `exporter.export-file(id, document, params, context)` | `exporters` | `file.export` and `model.read` | file bytes |
| `panel.render(id, context)` | `panels` | — | declarative view `{"blocks":[…]}` |

CAD IR, parameters and commands cross the boundary as JSON strings and are parsed by the host with the same strict types as its own Command API, so WIT stays stable while CAD IR grows. `context` carries `api`, `locale`, `revisionId`, `settings` and `idPrefix`; every new ID (feature, body, parameter) must start with `idPrefix` or the result is rejected.

## 12. Implementation notes

- **plugin-api 0.2** adds the `viewport` interface and the `viewport.capture` permission; the rest of the contract is unchanged. The WIT package keeps the name `formacad:plugin@0.1.0`, so 0.1 plugins keep working without a rebuild. A component that imports `viewport` must declare `"api": "0.2"`, otherwise installation fails with `PLUGIN_IMPORT_API_TOO_OLD`. FormaCAD 1.4.0 refuses such packages (`PLUGIN_API_UNSUPPORTED`), so they need `minAppVersion` 1.5.0 or newer.
- **Commands without `model.write`** (since 0.2): `contributes.commands` no longer requires `model.write`. Such a command may only return an empty `commands` array; any model-changing command is rejected as `PLUGIN_OUTPUT_INVALID`. The CAD app shows no model-edit approval for it.
- **Video recording** (`viewport.record-orbit`): the plugin passes a typed `orbit` record — frame size (even, 160–3840 × 120–2160), 1–120 fps, 0.5 s – 10 min, 0.05–100 turns, direction, the up axis (±X/±Y/±Z in CAD coordinates), tilt −90…90°, camera height −89…89°, start angle, zoom 0.25–4, background (theme/light/dark), `mp4` (H.264) or `webm` (VP9/VP8) and bitrate (0 = automatic); at most 36,000 frames. The host validates the values and returns an error string; one request per call. After the call the CAD app hands the UI a one-time ticket, opens the native save dialog, renders every frame of a copy of the current model on its own WebGL renderer (lights fixed to the camera, the user's camera untouched) and encodes the video with WebCodecs through the mediabunny library. Frames get exact timestamps, so the video is smooth even on a slow computer. Bytes are written to `<file>.part` at their offsets and renamed on success; cancelling or an error removes the partial file; the size limit is 4 GB. The plugin never receives the geometry, the pixels or the path.
- **Package:** only `manifest.json`, the `entry` file, the icon, `locales/*.json` and `assets/**` are allowed; stored/deflate compression; case-insensitive duplicate paths are rejected.
- **Catalog index entry** (`GET /api/v1/plugins/<id>/versions/<version>`): `{"payload":"<base64 JSON>","signature":"<base64 Ed25519 over the payload bytes>","keyId":"…"}`. The payload has `format: "formacad-plugin-index-entry/1"`, `id`, `version`, `name`, `permissions`, `sha256`, `size`, `downloadUrl`, `pageUrl` (both `https://plugins.formacad.ru/…`), `yanked`. The CAD app matches id/version with the link, the package size and SHA-256, and the manifest permissions with the signed list. Unknown fields are ignored.
- **Catalog key** lives in `apps/desktop/src-tauri/src/plugins/catalog/keys.rs`; it holds the production key `catalog-20261026` (the same bytes as `https://plugins.formacad.ru/api/v1/catalog/keys`). A build without keys answers `PLUGIN_CATALOG_KEY_MISSING` for catalog installs; file installs always work. Rotation: ship the new key next to the old one first, then switch the server.
- **A plugin operation** is stored as ordinary CAD IR operations (`add_feature`/`add_body`) in one revision; history shows “Plugin <name> <version>: <operation>”. The body therefore stays visible, measurable and exportable without the plugin. Reopening the operation with the plugin's parameters and pinning the version in the document are **not implemented**: that needs a new CAD IR operation type with worker support (proposal: `pluginFeature { pluginId, version, featureId, params, expanded }`).
- **Panels** open from the command palette as a dialog, not in the sidebar/inspector; a panel form submits a plugin command whose params match the form fields.
- **`formacad://` links** open a second window on Windows when the CAD app is already running (single-instance is not enabled, to keep multi-window and separate-profile launches unchanged).
- **Switching the active version** to one with new permissions is not done from settings: install that version again (consent window).

## 13. Not decided yet

- Author-side package signing (integrity currently comes from the hash and the catalog-signed index).
- Paid plugins — not planned.

## Verification

Checked by `npm run check:docs`. The manifest schema is JSON Schema draft 2020-12; the example manifest must validate against it (to be enforced by a test once code exists).
