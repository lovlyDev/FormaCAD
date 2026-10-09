# Imported STEP as a modeling feature

An exact STEP/STP source can be retained as the first typed CAD feature and modified by subsequent native operations. The original input stays an immutable project attachment. Translation, rotation, mirroring, Boolean operations, holes, whole-edge fillets/chamfers and the existing bounded selected-edge route operate on its actual BREP rather than an approximate reconstructed sketch.

## User workflow

Import a STEP/STP model into a project. Its new revision contains an `importStep` source feature and a named body. Open the model editor to inspect the operation chain; approved typed edits add modifiers above the import and update the body's output. Preview uses a separate worker workspace. Apply produces one validated revision. Export STEP contains the resulting geometry; full CADPACK and a self-contained project folder preserve the original input and authored operation chain.

An imported assembly currently enters this route as one aggregate named body. Its individual solids are not promoted into independently editable assembly components. Existing imported revisions without a typed document do not acquire a fabricated parametric sketch. Mesh imports remain meshes and cannot use this BREP feature.

## Contract and trusted input route

```json
{"type":"importStep","assetId":"step_<64 lowercase SHA-256 characters>","sha256":"<the same 64-character digest>"}
```

The reference contains no path, filename, URL or embedded file data. Its ID is derived from immutable content: replacing bytes creates a new identity. The host resolves the digest only among this project's STEP/STP attachments. Duplicate attachment names with identical content are equivalent inputs; a foreign project attachment, missing source or checksum mismatch cannot satisfy the reference. Original names remain user metadata and never become worker paths.

`native::assets::stage_project_assets` validates every authored reference and source byte count/checksum before writing immutable inputs under the isolated job's `inputs` directory. Only fixed digest-derived filenames reach the worker. The worker separately rejects traversal/links, nonregular or missing files, size changes and SHA mismatches before any geometry output. SHA verification streams through a 64 KB buffer. At most 32 distinct STEP sources are accepted, each at most 40 MB and all together at most 128 MB; repeated references to identical bytes count once. All authored inputs, including currently unused source nodes, must remain available for later body-output changes.

The dedicated native resolver opens the exact BREP, dependency execution rebuilds later operations, and the existing transaction persists only a successful validated candidate. A missing or changed input leaves the last saved revision untouched; it never substitutes the current preview mesh or another nearby geometry reference. `build_body` without an explicit asset workspace rejects an imported feature rather than reading a host path. Selected-edge selectors must uniquely resolve according to the existing geometric contract; unresolved or ambiguous topology fails instead of choosing a nearest edge. General persistent imported topology across arbitrary reimports remains unsupported.

## Persistence and limits

Source attachment bytes and SHA references travel together in immutable revisions and portable project exports. Redaction may omit optional attachments but must retain STEP inputs required by any included revision. STEP exports retain geometry only; the editable chain belongs to the Forma project. Removing a required source prevents future regeneration and must be reported explicitly. No new database schema is needed for the source feature; attachment storage remains content addressed.

Errors use `ASSET_INVALID`, `ASSET_MISSING`, `ASSET_CHECKSUM_MISMATCH` and `ASSET_LIMIT`, with localized display and separate technical details. A valid content hash does not make malformed STEP geometry acceptable: OCCT still parses and validates the BREP.

## Verification

`tests/imported_step_features.rs` creates an independent native box STEP, imports it through verified inputs, translates it, subtracts a cylindrical tool, checks analytic volume and STEP round-trip, applies a native fillet and rejects an unresolved selected edge after oblique rotation. Worker tests verify STEP/GLB publication and reject missing/tampered inputs without overwriting a saved artifact. Host tests cover project-scoped lookup, duplicate source content, immutable metadata, forbidden path fields, stale digests, failed command batches, 32-source/40 MB limits and unused-source validation. Packaging and real UI import/apply checks remain release-level verification.

See [modeling parity](modeling-parity.md), [commands](command-api.md), [projects](projects.md), and [project storage](project-storage-v2.md).
## Using the editor

New native imports immediately save an `importStep` CAD document. For an older revision without one, open Model editor and choose Enable native operations for this STEP: this stages a draft referencing the same verified SHA-256. It neither reconstructs old sketches nor replaces imported geometry with an invented profile. Only STEP/STP attachments with stored checksums qualify; STL and other mesh formats do not acquire fabricated CAD history. Preview and saving follow normal validation and append one revision.

Body rotation, reflection and profile revolution editors appear inside their feature cards. Their disclosures share the model editor animation and persisted expansion state. Axis/plane coordinates use mm, directions and normals are unitless, and angles use degrees. A temporarily zero direction remains editable in a draft, but native validation rejects saving it.
