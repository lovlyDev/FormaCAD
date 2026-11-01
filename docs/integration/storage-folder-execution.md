# Shared folder CAD execution / Общий CAD executor для папочного источника

Base inspected: `383fcf94e4b306e15cddd5a0dd708c5986d4f897` (task branch; the six target paths match the preparation copies).
Patch: [storage-folder-execution.patch](storage-folder-execution.patch).
Status: **applied after explicit lovlyDev approval for all six paths**. `git apply --check` succeeded before application against the task worktree. Only copies under `.local/storage-execution-preparation` were formatted with rustfmt; no production/shared file was edited and no build or test was executed by this preparer. Six source/test paths, 418 insertions and 144 deletions.

## RU

Это slice B для этапа 4: конкретная точка повторного использования настоящего CAD pipeline. Патч не подключает folder IPC, permission/task grants, writer lease, SQL commit или undo/redo. Он не доказывает готовность самостоятельного backend.

`modeling/build.rs` становится managed adapter: он передаёт прежний `AppState.root` одновременно как runtime root и transient root и использует прежний `artifacts::read`. Новый `modeling/execution.rs` содержит единственное перенесённое тело исполнения. `BuildRuntime` разделяет доверенный root поиска CadQuery/runtime и root временных model-builds; executable остаётся только прежним Rust host test seam, не параметром IPC. Существующий `Workspace::create` и `legacy::build` уже принимают явные root, поэтому их файлы менять не требуется.

`AssetSource` принимает настоящий `ProjectFile` и возвращает `Result<Vec<u8>>`. Folder host обязан предоставить ограниченный читатель, привязанный к исходному snapshot и immutable CAS, а не произвольный путь или голый hash. Callback удобно передать замыканием; `Send` сохраняет возможность выполнять общий async build в существующих задачах. Это доверенная Rust boundary, не сериализуемое право записи и не доказательство lifecycle само по себе. Исходный SQL/location/opening и совместимость ревизии всё равно проверяются orchestration после значимых awaits и перед публикацией.

Для imported STEP общий default `AssetSource::stage_native` вызывает новый `native::assets::stage_assets_with`. Этот единственный stager сохраняет document validation, список разрешённых STEP references, поиск descriptor только в captured Project, лимиты на один файл/суммарный размер, точную проверку size/hash и правило «проверить все inputs, затем immutable materialization». Worker, result metrics, bounded outputs и legacy fallback остаются исходными.

Managed adapter отдельно вызывает прежний `stage_project_assets`. Этот wrapper делегирует тому же stager, сохраняя managed-only filesystem preflight и классификацию `ASSET_MISSING`, `ASSET_LIMIT`, `ASSET_CHECKSUM_MISMATCH`. Общий callback не переклассифицирует ошибки: `PROJECT_SNAPSHOT_CHANGED` и recovery/location failures внешнего источника возвращаются исходными. Legacy base STEP сохраняет прежний выбор `program_base`/`source`, имя `base.step` и исключение typed programs.

Семь новых синтетических регрессий подготовлены:

1. Legacy base читает точный captured descriptor через callback без managed storage, сохраняет source и точные staged bytes.
2. Typed program не читает legacy base.
3. Stale callback error возвращается без materialization и переклассификации.
4. Отсутствующий base descriptor отклоняется до callback.
5. Imported STEP использует общий stager и точный descriptor без AppState/app DB.
6. Stale imported STEP сохраняет исходный lifecycle error и не пишет input.
7. Oversized descriptor отклоняется до callback; неверный hash отклоняется до materialization.

Последние три теста требуют `native-worker-client`; они проверяют staging synthetic bytes, не валидность STEP и не geometry. Прежний typed-v1/native-v2 routing test перенесён вместе с исполнением без изменения assertions. Присутствие тестов не означает PASS.

### Точные пути для отдельного согласования

1. `apps/desktop/src-tauri/src/modeling.rs` — регистрация crate-visible execution module.
2. `apps/desktop/src-tauri/src/modeling/build.rs` — managed adapter и прежний GeneratedModel (видимость расширена до crate для общего executor).
3. `apps/desktop/src-tauri/src/modeling/execution.rs` — перенесённый единственный executor, runtime и asset seam.
4. `apps/desktop/src-tauri/src/modeling/execution/source_tests.rs` — семь synthetic input regressions.
5. `apps/desktop/src-tauri/src/native/assets/stage.rs` — общий callback stager + сохранённый managed wrapper.
6. `apps/desktop/src-tauri/src/native/assets/mod.rs` — re-export общего stager.

**Пути 5–6 принадлежат общей native-области Issue #3. lovlyDev отдельно разрешил все шесть файлов после регистрации scope в Issue #31; применён именно этот патч.** Это явная граница по §5 ТЗ, не запрос на слияние или выпуск.

После разрешения main должен проверить `git apply --check`, применить точные шесть путей, выполнить rustfmt/Clippy и действующие generic/native host suites. Нужны настоящие managed apply/imported STEP tests, включая прежние error classifications и metadata-only saves. Затем folder orchestration должен выполнить реальный worker с snapshot-bound reader и отдельно проверить stale/opening/commit guards. Ни Cargo/C++/worker protocol, ни permissions/commands/SQL/UI/версии этот патч не меняет.

## EN

This is applied slice B following explicit owner approval, not a completed folder backend. It extracts the existing real CAD executor into one `modeling/execution.rs` implementation. The managed wrapper supplies the same root for trusted runtime discovery and transient workspace storage. The existing Rust-only executable test override remains host-owned; no new IPC capability is introduced.

`AssetSource` reads a captured `ProjectFile`; a folder host must bind its bounded reader to the original snapshot and immutable CAS. Its default native staging path calls `stage_assets_with`, retaining the existing document/reference/descriptor/size/hash/aggregate checks and validate-all-before-materialization order. The managed override calls the existing wrapper, whose preflight and error classification remain unchanged. Shared callback errors propagate unchanged so stale/recovery errors are not disguised as missing assets or checksum failures.

Legacy base selection, typed-program bypass, actual native worker calls, legacy fallback, ReviewPlan metrics and bounded output checks retain one implementation. Runtime, source lifecycle, permission/task authority, durable SQL publication and uncertainty handling remain distinct host responsibilities; this extraction provides none of those guarantees by itself.

The exact six paths are listed above. The two native asset files are owned/shared Issue #3 points; lovlyDev separately authorized this exact six-file scope before application. No Cargo, C++, protocol, version, permission, SQL, command or UI files are changed. Applicability checking passed before application. Preparation copies were formatted first; root subsequently ran cargo fmt after applying the authorized scope. No test/build success is claimed.

Seven new synthetic tests cover callback descriptors, legacy/typed routing, stale-error preservation, missing descriptors, imported staging and size/hash refusals. Three require `native-worker-client`. Existing routing assertions move unchanged with the executor. Real managed/native host checks and a real folder-worker vertical test remain necessary after approval and application.
