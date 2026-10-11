# Forma CAD 2.x — shared implementation plan

Status: agreed product direction and implementation plan, 2026-09-23; shared for sequential development on two PCs, 2026-10-11. This document defines scope and acceptance, not a claim that 2.0 is ready. Current verified results and pending work are recorded in docs/ru/status.md and docs/en/status.md.

## Русская карта плана

**Назначение.** Forma CAD 2.0.0 — рабочий архитектурный выпуск, а не вся линия 2.x за один раз. Основа продукта — локальный проект, собственное CAD-ядро, единые типизированные команды, проверяемые и отменяемые изменения, AI как способ управления той же моделью. Раздел 229 исходного ТЗ относится к завершённости всей линии 2.x. Номера будущих minor-выпусков можно менять до их выхода.

**Основной пользователь и путь.** Новичок или практик создаёт корпус для печати либо небольшую точную механическую деталь. AI уточняет критические размеры, показывает план с допущениями, пользователь подтверждает. Forma выполняет план одной транзакцией, проверяет геометрию и размеры, сохраняет ревизию. Пользователь выбирает грань или ребро, измеряет их, просит AI изменить выделенное, меняет именованные размеры вручную, сравнивает версии и экспортирует. Без AI доступны импорт, шаблоны, простые тела, параметры, измерения и экспорт.

**Основа 2.0.** Сохраняем Tauri/React/Rust, заменяем обязательный Python/CadQuery runtime отдельным локальным Rust/C++/OpenCascade CAD-процессом. Rust общается с C++ через `cxx`. Все изменения из UI, AI и будущих Lua/WASM проходят через Command API и CAD IR. Существующие пользовательские возможности моделирования и реальные геометрические операции переносим; синтаксис прежнего CadQuery-кода сохранять не нужно. Сбой CAD-процесса не закрывает Forma и не публикует неполную модель.

**Граница 2.0.** Включаем Codex, Claude и Custom CLI с обязательным JSON-контрактом; локальный AI-провайдер — в раннем следующем minor. В 2.0 есть числовые параметры с единицами, именованные тела, базовое дерево операций, выбор граней/рёбер и AI-контекст выделения, точные измерения, сечение, X-Ray/Ghost, WebGPU с функциональным WebGL2 fallback, ревизии/undo/redo, сравнение, `.cadpack`, восстановление, строгий новый интерфейс. Формулы параметров, перетаскивание граней, полноценные эскизы, сборки, чертежи, Lua, WASM, производство, симуляция и ветки добавляются позже. Пустые разделы будущих функций не показываем.

**Интерфейс.** Сверху компактные группы инструментов; слева «Модель / Файлы / Параметры», справа «AI / Свойства», снизу сворачиваемые «История / Задачи / Активность / Логи». Проекты открываются вкладками. Видно, какая геометрия прикреплена к AI-сообщению. Панели меняют размер и запоминают состояние. Нейтральное оформление со сменным акцентом, две темы, русский по умолчанию, английский, настраиваемая плотность, масштаб текста и горячие клавиши. Качество отображения автоматически подстраивается под ПК, при движении камеры временно упрощается, но точность CAD и измерений не меняется.

**Проекты.** Каждый новый проект — переносимая папка; общие данные приложения содержат список последних проектов и настройки. Внутренний формат метаданных выбираем после замеров скорости и проверки восстановления, а не заранее по названию БД. Есть блокировка записи при втором открытии, режим чтения с возможностью сохранить копию, резервная копия перед миграцией, локальные автоматические копии, журнал восстановления. `.cadpack` даёт полный пакет или настраиваемую копию для передачи. Старые проекты 1.1.0 остаются нетронутыми и показываются отдельно; из STEP создаётся новый 2.0 проект без обещания восстановления старой истории.

**Переносной режим.** Обычная установленная Forma может переключиться на выбранную папку данных на внешнем носителе. Для запуска на другом компьютере нужна отдельная portable-сборка под его ОС. Пропадание NAS/USB останавливает новые записи, сохраняет черновики локально и требует проверить исходную папку перед продолжением.

**Гарантии изменений.** Один одобренный AI-план — одна транзакция и один шаг Ctrl+Z. При ошибке откатывается весь план; AI может дважды попытаться исправить результат в пределах тех же требований. Изменившиеся требования снова показываются пользователю. Несколько ручных изменений параметров применяются одной транзакцией; геометрический предпросмотр запускается по кнопке. Неоднозначная ссылка на грань или ребро вызывает видимый конфликт и ручной выбор, а не тихую подмену объекта. Внешние файлы и действия учитываются отдельно: отмена модели не обещает удалить уже экспортированный файл.

**Очередь и проверка.** Одновременно выполняется одна тяжёлая CAD-задача, остальные ждут; AI-планирование идёт отдельно. Пока модель строится, можно просматривать последнюю сохранённую версию и работать с другим проектом. После построения показываем проверенную геометрию, подтверждённые размеры и требования, которые пока нельзя проверить автоматически. Ошибки содержат код, затронутую операцию и способ исправления.

**Порядок реализации 2.0.** (1) инвентаризация реальных возможностей 1.1 и тестовых деталей; (2) схемы CAD IR/Command API/проекта/обмена с worker; (3) отдельный C++/OCCT worker и пакетирование на три ОС; (4) перенос геометрических операций; (5) транзакции, переносимые проекты, ревизии, recovery; (6) AI-адаптеры и разрешения; (7) выбор, измерения и два графических пути; (8) новый интерфейс и локализация; (9) сквозные проверки, установщики, подпись и выпуск. Подробные контракты, проверки и критерии находятся ниже.

**Линия после 2.0.** Предлагаемый порядок: параметры и формулы → первый локальный AI → полное дерево и перестроение → эскизы → связи эскизов → прямое редактирование → сборки → сопряжения/BOM → углублённый просмотр → чертежи → Lua → WASM → UI/SDK плагинов → материалы и производство → анализ печати → CNC → локальная симуляция → расширенный AI-контекст → роли агентов → Design Review → оптимизация → большие сборки → усиление безопасности → доводка. Ветки получают отдельный выпуск после стабилизации ревизий. Конкретные номера и состав этапов см. в таблице ниже; они допускают разделение и перестановку до релиза.

**Порог выпуска.** 2.0 можно передавать пользователям только после работы офлайн-ядра без Python и сервера разработчика, успешного сценария «AI → план → подтверждение → модель → измерение → отмена → экспорт», проверки отказов и восстановления, импорта STEP и сеток с честным обозначением их типа, работы на Windows/macOS/Linux, полного WebGL2 fallback, тестов безопасности, миграций и установщиков. Версии npm workspace/lockfile, Cargo и Tauri синхронизированы; старые релизы не перезаписываются. Публикация и установка — отдельные действия по указанию пользователя.

**Что ещё предстоит проверить технически.** Бенчмарки формата проекта на SSD/NAS, точность и паритет операций OCCT, условия распространения CAD-библиотек, реальную поддержку WebGPU в целевых webview, платформенные ограничения portable-сборки, первого локального AI-провайдера и первого решателя симуляции. Это задачи проверки, а не повод откладывать уже согласованные продуктовые решения.

## 1. What this plan means

Forma 2.0.0 establishes a usable independent local CAD platform. The full MASTER SPEC is the destination of the 2.x line, not a requirement to implement every item in 2.0.0. Section 229 of the original spec is interpreted as a long-range completion checklist for 2.x. Future minor release numbers may change before release. Patch versions handle fixes and small compatible improvements. No labels such as alpha, beta, preview, prototype, or MVP appear in product releases.

Primary audience: beginners and practical makers of printed enclosures and small precise mechanical parts. Primary journey: ask AI to create a model, inspect it, edit named dimensions or operation properties, select a face or edge, ask AI to change the selected geometry, measure, compare revisions, and export. Manual creation from templates and simple primitives works without AI.

Hard constraints: local projects and data; no developer service, account, cloud database, telemetry, or mandatory network for CAD work; typed model changes from all sources; deterministic validation; reversible successful changes; no silent changes to old projects. Windows is the priority, with Windows, macOS Apple Silicon/Intel, and Linux builds required for 2.0.0. The installer bundles the CAD runtime; third-party AI CLIs remain optional external installations. Python is absent from the required 2.0 CAD runtime.

## 2. Verified baseline and migration implications

Checked in the local tree before writing this plan:

| Area | Current code | 2.0 direction |
| --- | --- | --- |
| Desktop | Tauri 2, React/TypeScript, Rust, SQLite, Three.js | Preserve app identity and working release pipeline; replace workspace UX and add a native CAD process |
| CAD representation | Rust/TS JSON document version 1; stable feature IDs; rectangle, circle, extrude, translate, boolean | Versioned CAD IR with typed commands, multi-body model, named parameters and persistent geometric references |
| Geometry | Bounded Python/CadQuery worker produces STEP and GLB; additional compatibility syntax supports far more geometry than typed document | Bundled C++20/OCCT worker reached from Rust via cxx; user-visible modeling operation parity verified on representative models |
| AI | Codex/Claude native process launch, structured final response, limited progress events; compatibility fallback to bounded CadQuery | Provider adapter API, structured tool/events, typed plan and atomic execution; Custom CLI with required JSON contract |
| Project | Shared application SQLite is authoritative; sidecar project JSON and immutable revisions | Portable self-contained project folder; benchmark storage layout before freezing schema; application database remains a recent-project/settings index |
| History | Append-only revisions, restore by new revision, some comparison UI | Command transaction log, plan-level undo/redo, revision comparison with synchronized side-by-side and overlay |
| Viewer | Three.js/R3F, existing render modes and camera controls | WebGPU where verified; full WebGL2 fallback; topology selection, exact kernel measurements, section and X-Ray/Ghost |
| Distribution | Multi-platform CI and signed updater configuration exist | Bundle CAD dependencies on all platforms; verify full installers, signed update metadata, offline core, recovery |

Current development is consolidated as Forma 1.2.6. Native CAD, typed editing, sketch constraints, reviewed AI candidates, project history, transfer packages, committed snapshots and supported topology measurements are implemented. The complete architectural 2.0 acceptance criteria below remain binding; current Windows tests do not establish installed, cross-platform, removable-media or NAS readiness. See docs/en/status.md for current implementation limits. Applied SQL migrations remain immutable.

Existing 1.1 project data need not be migrated into editable 2.0 history. Preserve the old data. Dashboard shows a separate “previous projects” section and offers creation of a **new** 2.0 project from the last saved STEP body, explaining that original operation history is not recovered. Never silently overwrite or delete old storage. Within the 2.x line, migrations are required: make backup, detect schema version, migrate into staging, validate, atomically commit, and roll back on error.

## 3. Non-negotiable architecture contracts

### 3.1 Boundaries

`React UI → typed IPC → Rust application/project/permission services → Command API → CAD IR transaction → CAD worker (Rust → cxx → C++20/OCCT)`. The renderer owns interaction and transient visual state. Rust owns permissions, file access, tasks, serialization, revisions, locking, AI adapters, and the transaction coordinator. The worker owns BREP operations, STEP/IGES translation, topology, validity, tessellation, mass geometry, and precise measurements. Never let C++ exceptions cross the FFI boundary. The CAD worker is a separate local process; a worker crash does not terminate the UI.

One heavy CAD job runs at a time in a visible cancellable queue. AI planning can run separately. While a model transaction executes, the last committed model remains viewable and measurable, and other projects remain usable; modifications of the busy project wait. Cancellation requests a cooperative stop, then kills the worker after a bounded grace period. A failed or cancelled job leaves the committed revision unchanged.

Use typed control messages with request IDs and structured errors. Transfer large meshes through binary buffers/files or shared memory with explicit lifetime and checksum, not JSON arrays. Version the worker protocol and CAD IR separately. Keep Rust/C++ boundary calls bounded and typed. Cache derived meshes/previews by content hash; cache deletion must never destroy source geometry or project history.

### 3.2 CAD IR and commands

Define versioned schemas for Document, Body, Feature, Sketch, Parameter, SemanticReference, Command, Plan, ValidationFinding, Revision, and Selection. IDs are opaque stable identifiers, separate from display names and array positions. At minimum establish project, part, body, feature, sketch, parameter, face, edge, and vertex IDs, plus assembly component IDs reserved for future use. Preserve unknown future fields only under an explicit compatibility policy; reject unsupported modifying commands.

A modifying command specifies its source, expected revision, target semantic ID, typed arguments with units, and permission scope. The command validator checks schema, allowed ranges, dependency existence, reference type, permission, revision conflict, operation preconditions, and resource limits before invoking the kernel. A plan is an ordered list of commands plus assumptions, affected bodies, declared requirements, and expected checks.

One AI plan is one transaction and one Ctrl+Z step. One manual batch of parameter edits is one transaction. A later macro run is one transaction by default. Execute against staged model state; rebuild; validate BREP and semantic references; generate a human-readable diff; commit revision, geometry, and index consistently. On failure, reject all staged changes. A restore creates a new revision. Redo replays the validated command record or restores a verified successor state under explicit rules; UI snapshots are not the source of truth.

Significant AI-created dimensions become named project parameters with stable IDs. Minor operation-specific values remain local properties and may be promoted later. In 2.0 parameters are numeric with units; expressions, derived values, and cycle detection enter in 2.1. The internal geometric tolerance is stable and independent of UI display precision and tessellation. Units and display precision are per project; input accepts explicit units and converts without rounding model geometry.

Semantic face/edge references carry creation feature, geometric signature, adjacency/orientation and provenance, rather than raw tessellation index. On rebuild, map only unambiguous references. If mapping is ambiguous or absent, stop dependent operations and show a conflict with candidate objects for manual reselection. Keep the last committed body accessible until the staged transaction validates.

### 3.3 Current modeling parity gate

Inventory every user-visible geometry path in 1.1: six templates, typed operations, and compatibility CAD methods. Separate an actual user scenario from a syntactic alias in CadQuery. Build a parity matrix with an input request/model, intended BREP outcome, new typed command sequence, measurement assertions, STEP round-trip, and representative negative case. The new core must preserve practical operations for enclosures and mechanical parts, including the currently usable advanced operations, before removing the Python runtime. Old CadQuery source syntax itself is not a 2.0 compatibility requirement.

Do not quietly route unsupported CAD requests to arbitrary code. An unsupported request returns a clear capability error and, where possible, a plan using existing commands. Imported STEP preserves exact BREP bodies and part structure; new operations can be placed above the import node. STEP does not imply recovered parametric history. Mesh imports are marked as meshes and allow viewing, mesh-based measurements and export; BREP-only edits remain unavailable until a later explicit conversion workflow.

## 4. Concrete 2.0.0 product scope

### 4.1 Projects, storage and recovery

- Create/open self-contained ordinary project folders in a user-selected directory. Keep a recent/pinned index in app data. Open multiple project tabs in one window; each remembers camera, selection, right-panel tab, chat draft, and panel dimensions. Default startup opens Dashboard; optional setting restores tabs.
- Select the on-disk metadata/index format using benchmarks: cold open, large feature graph, long chat, many attachments, local SSD, shared folder, interrupted write, backup/restore, and archive portability. SQLite inside each project plus files is a candidate, not a pre-decided mandate. Publish internal schema and migration fixtures once chosen.
- Use an OS-level cooperative lock with process identity and heartbeat where practical. A second opener is read-only and may “Save a copy”. Detect stale locks conservatively. On lost NAS/USB access, stop writes, save drafts to local recovery storage, then recheck for conflicting changes before resuming or creating a new copy.
- Autosave only successfully committed model transactions. Recover last committed revision after crash and offer to restore chat/parameter drafts; never automatically replay interrupted AI or CAD work. Keep a recoverable journal of incomplete tasks and transaction intent. Backup before every project migration; optionally schedule local rolling backups to a chosen folder.
- Define `.cadpack` as a versioned, bounded portable package with manifest, hashes, explicit file list, and safe extraction. “Full project” includes history, chats and attachments; “copy for sharing” permits excluding optional data. Required geometry and dependency files always accompany included model state. Credentials and machine-level permission grants are never exported. Imported plugins/macros are inactive until separately reviewed and enabled.
- Installed Forma may switch to a selected portable data directory via checked move/copy and next-start handoff. A separate portable executable bundle runs from an external disk on another computer. No write assumption next to an installed EXE.

### 4.2 AI and permission UX

- Codex, Claude, and Custom CLI are required 2.0 adapters. Custom CLI has executable path, argument array, cwd policy, timeout, JSON request/response contract, detection, cancellation, stderr/stdout streaming, and structured errors. Do not pass shell command strings. Adapter capabilities declare image/tool support.
- AI asks for critical missing dimensions, shows all other assumptions in the plan. The plan preview lists operations, dimensions, affected bodies, assumptions and expected checks. User approval authorizes that plan. Changed intent or requirements require a new approval. On failure, allow at most two correction attempts within the authorized scope; each attempt starts from the same committed revision. If still invalid, roll back the entire plan and show structured failure.
- AI receives current revision and explicit selection context including semantic IDs, type, feature origin and measured geometry. A visible attachment to the message shows what selection is sent. If a reference no longer resolves, ask for reselection. Multiple named conversations share one current project model. Plans bind to a revision and are revalidated after any intervening edit.
- Project memory is an editable list of user-confirmed goals and decisions. AI suggests entries; suggestions require confirmation. Conversation summaries are separate and cannot silently become engineering requirements. A detected requirement conflict is resolved before building. Distinguish deterministic requirement checks from AI assessments.
- Explicitly attached files are shown before external provider transmission and checked against provider format support. Do not automatically send all project files. Switching provider in a conversation shows what context will be transmitted and retains attribution of older messages. The developer does not receive project contents or prompts by default.
- Agent tools exposed by Forma allow bounded reads, measurements, reference inspection and command preparation. Modifications execute in the host after plan approval. Filesystem, process, external path and network capabilities require separately scoped permissions. Grants support once, session, and project, keyed by actor and action, visible/revocable in Settings with an audit trail. A prompt alone is not a sandbox: enforce boundaries in the launched process and host APIs.
- After a successful build, show completed operations, verified geometry and dimensions, and requirements not verified. Do not label an AI judgment as a kernel measurement. A major failure provides code, affected feature/reference, explanation and recovery action.

### 4.3 New desktop workspace

- Redesign as a strict engineering interface: neutral surfaces, thin borders, modest radii and user-selectable accent. Separate semantic colors for selection, warning, error and measurements. Russian is default; retain persisted locale, translate every new UI string into ru/en through i18n, format numbers/dates by locale, and support light/dark themes including live switch without scene reset.
- Dashboard: recent/pinned projects, search, templates, create/open/import CAD or `.cadpack`, previous-version projects, recent exports/revisions. Short skippable first-run setup covers project location, units, theme and AI detection. No account step.
- Main window: project tabs; compact grouped tools above viewport; left tabs Model/Files/Parameters; right tabs AI/Properties; lower collapsible History/Tasks/Activity/Logs, normally closed; persistent status bar. Panels resize/collapse and save per-project layout. Right side never automatically switches from AI when geometry is selected. Selection appears beside the chat composer; Properties updates if already open.
- Model tree selects bodies/features in the viewport, supports names, operation parameters, dependencies and errors. Preserve immutable feature IDs on rename. Suppress/reorder/dependency-aware deletion enter later history release.
- Hover highlights candidate geometry. Selection filters: bodies, faces, edges and vertices. Multi-select supports measurements and AI actions. Context actions near selection show valid commands. Keyboard mapping is editable with search, conflict detection and reset. Text entry consumes its own shortcuts. Command palette searches commands, settings and current-project bodies/features/parameters/revisions.
- Default mouse control: drag empty viewport with left button to orbit; click model to select; right click menu, right drag pan, wheel zoom. Offer remapping. Settings provide standard/compact/large density, separate text scale, UI/theme controls, and performance auto/manual settings. Notification sounds are enabled softly by default; click sounds are off but configurable. No sound on orbit, scroll or continuous dragging.

### 4.4 Viewer and geometry inspection

- Prefer WebGPU after capability check; keep full WebGL2 implementation for all core 2.0 selection/inspection functions. Implement shaded, shaded+edges, wireframe, transparency, X-Ray and Ghost. X-Ray exposes inner structure; Ghost dims context around selected bodies.
- Exact measurements through the BREP kernel: point-to-point distance, edge length, radius, diameter, angle, face area, body volume, bounding box. Show mesh measurements distinctly as mesh-derived. Temporary measurements can be pinned. Pinned measurements recalculate on rebuild, and unresolved references become visible errors; they are not dimensional constraints.
- Section plane supports XY/XZ/YZ, selected planar face, offset, numerical orientation and free rotation. GPU clipping responds during drag. A separate command computes exact OCCT intersection geometry for measurement; invalidate that contour when plane or model changes.
- Viewer camera modes, fit, standard views, projection, grid, axes, body visibility and selection persist. Tessellation runs off the UI thread. During camera interaction, automatic performance mode can temporarily reduce visual detail and restore it afterward; exact model and measurement results never change. User can override performance policy. Test on varied hardware instead of baking in one RAM/GPU threshold.
- Revision compare supports side-by-side synchronized cameras, color overlay, and semantic list of changed parameters/operations. Geometric deviation heatmaps belong to a later release.

### 4.5 Manual modeling and import/export

- Retain current templates and add simple dimensioned primitives such as block, cylinder and sphere, all through the same IR. Allow multiple named bodies, selection, hide/show, measurement and export of individual bodies. Full assembly mates are later.
- Edit multiple numeric parameters in a draft; offer “Preview” as an explicit trial rebuild, and “Apply” for a validated commit even without preview. Enter applies, Escape discards draft. If preview is current, reuse its validated geometry on apply where safe. One batch is one undo step.
- Single export dialog chooses whole model or selected bodies, format, units and tessellation quality; remember settings per project. Keep current STEP, STL, 3MF, OBJ and GLB workflows in parity inventory. Mark precise BREP versus mesh export clearly. Export does not mutate model state.

## 5. Execution sequence to reach 2.0.0

The following are implementation slices, not public product labels. Each slice leaves a runnable app and has an explicit validation gate.

1. **Baseline and parity inventory.** Reconcile version files before the next actual release. Record real 1.1 templates, typed document operations, compatibility CAD methods, import/export cases, AI provider behavior, platform builds, and existing fixture models. Establish golden geometry and STEP round-trip cases. Keep prior releases unchanged.
2. **Versioned contracts.** Define CAD IR v2, Command API v2, source/actor identity, plan schema, semantic selection/reference types, structured errors, worker IPC and project package manifest. Publish cross-language generated types and round-trip tests internally. Freeze only after example enclosure, bracket, imported STEP and multi-body documents fit.
3. **Native CAD process.** Add CMake/Ninja C++20 OCCT integration, Rust cxx bridge, bounded exception boundary, worker launcher, timeout/cancel/crash isolation and binary mesh transfer. Bundle redistributable dependencies and verify licenses/platform packaging. Validate operations on all target OS CI runners.
4. **Geometry parity.** Implement typed construction, transforms, booleans, profiles, and advanced operations needed by actual 1.1 modeling scenarios. Each operation returns BREP, topology provenance, validation and tessellation. Add numerical/degenerate fixtures and measure against expected geometry. Remove Python runtime dependence only after passing the parity matrix.
5. **Transactional project engine.** Implement independent folder projects, chosen storage layout, lock protocol, command staging, atomic commit, revision lineage, plan-level undo/redo, autosave and recovery. Exercise crash at every commit stage and migrate v2 schema through versioned migrations. Add previous-version project discovery and copy-from-STEP.
6. **AI execution.** Define Codex/Claude/Custom adapters, normalized events, project context and selection, typed plan validation, approval, bounded correction attempts and verified result report. Prove that unsupported operations do not execute arbitrary source and stale plans do not change a newer revision.
7. **Viewer foundations.** Build BREP-backed semantic selection, exact measurements, pinned reference behavior, WebGPU plus WebGL2 parity, binary mesh loading, section/X-Ray/Ghost and adaptive rendering. Verify light/dark theme switch without geometry/camera reset.
8. **Workspace and project UX.** Replace layout with dashboard/tabs/tree/AI/properties/lower panel, command palette, shortcut editor, i18n, permissions, settings, drafts and sound controls. Include screen-reader labels, keyboard reachability and error states.
9. **Packaging and acceptance.** Build offline Windows installer and portable package plus macOS arm64/x64 and Linux packages with CAD runtime. Validate signed updater metadata using CI secrets, never checked-in private keys. Run platform smoke scenarios, update recovery, project/package corruption, security and performance tests. Publish only after user instruction and release gate; local builds are already authorized by AGENTS.md.

## 6. 2.0.0 acceptance scenarios

Each scenario must pass without a developer server. Offline AI execution is expected only after a local provider is added later; offline CAD functionality is required now.

1. Fresh install with network disabled: launch, choose project folder, create block/plate/enclosure, edit parameters, undo/redo, measure, export STEP/STL and reopen project.
2. Select a face and edge on an AI-created multi-body enclosure; the UI shows stable IDs and exact properties; ask AI for an edit using selection; inspect typed plan; approve; verify changed feature and measured dimensions; undo the whole plan.
3. Execute a multi-command plan with a failing last operation; confirm that neither body, revision pointer nor pinned measurement is partially committed. Retry at most twice within unchanged intent.
4. Import a STEP assembly/part; preserve exact BREP and body identity; measure; add a typed feature on top; export STEP; reopen and compare geometric invariants. Import an STL/OBJ/GLB and verify explicit mesh semantics.
5. Deliberately create an ambiguous topology reference, broken dependency, zero/negative dimension, invalid Boolean and malformed CAD input. Show structured errors and keep the last valid model.
6. Kill the CAD worker during a heavy operation; keep UI responsive, recover worker and last committed model. Kill app during staging; restore latest committed model and optional drafts, without rerunning AI.
7. Open same project from two processes, including a shared folder; second is read-only with “save copy”. Disconnect shared drive mid-task; prohibit writes and offer recovery/copy.
8. Export/import full and redacted `.cadpack`; check hashes, path traversal, archive bombs, excluded credentials and inactive imported extensions. Validate round-trip on another supported OS.
9. Verify both WebGPU and WebGL2 paths for selection, measurements, section, X-Ray/Ghost, keyboard navigation, Russian/English and light/dark switching.
10. Install/update on Windows, macOS and Linux with bundled CAD core; launch without Python/CadQuery. Verify updater failure leaves offline core usable and pre-migration backup restores a project.

Release gate: no critical blockers; schema migrations and recovery fixtures pass; applicable security tests, i18n check, migrations check, frontend/Rust/C++ tests, formatter/lint, three-platform CI and installer smoke tests pass; docs/release notes match shipped behavior; all npm workspace, lockfile, Cargo and Tauri versions match; prior versioned release artifacts remain intact. Check artifact existence and record a new EXE path when making application changes for a Windows release. Actual publication or installation requires separate user instruction.

## 7. Proposed compatible 2.x release line

Numbers after 2.0 are planning slots, not promises. Split a slot if its acceptance gate is too large; do not ship a partly implemented named feature to preserve numbering. Do not reserve 3.0 merely because the minor number exceeds 9. Every release requires compatible project loading or a tested migration, offline core validation, docs/release notes, CI, installer checks, and regression of earlier capabilities.

| Proposed release | Working focus and complete outcome | Dependencies and acceptance |
| --- | --- | --- |
| 2.1.0 | Parametric expressions: named/derived parameters, unit-aware expression parser, dependency graph, cycle diagnostics and selective rebuild | Builds on 2.0 IR; edit one driving value, rebuild affected features deterministically, undo as one transaction |
| 2.2.0 | Local AI provider I: one verified adapter selected after checking installation, protocol and platform support; provider capabilities shown in settings | Same typed-plan, selection, permission and result checks as CLI adapters; runs without internet |
| 2.3.0 | Full feature history: suppress/unsuppress, dependency visualization, statuses, partial rebuild, safe deletion/reordering where valid, semantic diff | Broken references produce conflicts, not wrong edits; regression against 2.0 topology selection |
| 2.4.0 | Sketch drawing: line, polyline, rectangle, circle, arc, spline, construction lines, grid/snapping, trim/extend/offset/mirror | Sketch entities have stable IDs; automatic constraints show hints and may be disabled |
| 2.5.0 | Sketch constraints and dimensions: replaceable solver, constraint health, conflicting constraints, AI sketch commands through Command API | Valid under-constrained sketches may build with warning; over-constrained/invalid sketches block dependent build |
| 2.6.0 | Direct geometry editing: selected face/edge manipulator, drag and exact numeric input in one tool, staged live result, confirmation/escape | Converts interaction to feature/command, preserves history and reference mapping; consistent with AI edits |
| 2.7.0 | Assembly foundation: parts, instances, transforms, internal bodies and external project references pinned to revisions, visibility/isolation/explode | Multiple instances share a definition; `.cadpack` includes needed pinned dependencies |
| 2.8.0 | Assembly constraints, interference and BOM: mates, degrees of freedom, broad/narrow checks, component metadata and exports | Conflicting new mate rejected, existing solved position retained and conflicting mates identified |
| 2.9.0 | Advanced inspection and engineering overlays: selection refinements, curvature/normals/thickness, measurement ergonomics, additional WGSL effects | All essential actions retain WebGL2 fallback; computed quantities expose method/precision |
| 2.10.0 | Technical drawings: orthographic/isometric/section views, associative dimensions and annotations, title blocks, PDF/SVG/DXF export | Start with validated ГОСТ/ЕСКД and ISO templates; changed model marks drawing stale; user triggers rebuild; broken dimensions shown |
| 2.11.0 | Lua automation: sandboxed Lua 5.4 host, `.cadmacro`, editor, import/export, settings UI, logs, recording, AI generation | One macro run is one CAD transaction; model rollback cannot claim to reverse external exports |
| 2.12.0 | WASM plugins: `.cadplugin`, Wasmtime sandbox, import/update/remove, capability grants, resource limits, plugin commands **and custom parametric feature types from the first plugin release** | Versioned feature schema, pin plugin version per project, allow parallel installed versions; missing plugin preserves cached geometry but blocks dependent rebuild |
| 2.13.0 | Declarative plugin UI and SDK: host-rendered settings/panels, context actions, overlays, validated shaders and Rust/C++/Zig/TS-oriented examples | No arbitrary React/HTML/JS in host UI; compatible API versioning and migration path |
| 2.14.0 | Materials and manufacturing foundation: built-in reference records, custom material import, process profiles, general checks and advisor | Project stores property snapshot with source/conditions; no universal “manufacturable” verdict without process profile |
| 2.15.0 | 3D-printing checks: walls, overhang, orientation, support estimate, bed fit, mass/volume and print-time estimate | Findings trace to material/printer profile and geometry; distinguish estimate from measurement |
| 2.16.0 | CNC checks: accessibility, cutter/corner fit, tool reach, pockets and warnings | Findings use actual tool/process profile and identify uncheckable cases |
| 2.17.0 | Local simulation I: one fully verified local solver adapter and linear static analysis of one part | Material/loads/fixed faces/mesh/results versioned; stress, displacement and safety display; changed inputs mark result stale |
| 2.18.0 | Project-aware AI II: richer viewport/screenshot/feature/revision context, local summaries and attachment handling | Existing explicit file-transmission and confirmed-memory rules remain in force |
| 2.19.0 | Multi-agent roles: sequential Design/Validation/Manufacturing/Review with per-role provider selection and deterministic host checks | No AI pass/fail substitution for available kernel/analysis checks; context transfer shown |
| 2.20.0 | Design Review: traceable findings for geometry, walls, collisions, fasteners, materials, printability and verified dimensions | Each claim labelled verified, estimated or unverified; repairs use approved typed plans |
| 2.21.0 | Design variants and optimization: goals, constraints, bounded time/attempt budget, local scored variants and comparison | Variants stay separate; user chooses if one becomes main model; scores cite measured evidence |
| 2.22.0 | Scale and large assemblies: hierarchical culling, LOD, instancing, deduplication, lazy loading, background tessellation, cache/binary IPC tuning | Repeatable benchmark sets, memory budgets and acceptable interaction latency across GPU tiers |
| 2.23.0 | Security hardening: project/AI/Lua/WASM isolation, malformed packages and CAD, timeout/cancel, path and process abuse, recovery attacks | Adversarial suite and regression of permission boundaries across all supported OSes |
| 2.24.0 | Production polish: migration and recovery matrix, accessibility, installer/updater/startup quality, keyboard and documentation cleanup | Full 2.x compatibility audit and platform acceptance |
| 2.25.0+ | Follow-on local integrations: other local AI providers, additional simulation solvers, local slicer/CAM adapters, optional equipment workflows | Each integration uses existing permissions, typed commands, offline core and explicit confirmation before equipment execution |

Local branches fit after the 2.0 revision format stabilizes; create a dedicated compatible minor release rather than burying them in a patch. First branch release supports local branches, visual diff and controlled merge: merge independent parameter/operation/metadata changes; show conflicts for manual resolution; never claim universal geometric merge. All merged output is rebuilt and validated. Additional solver/provider releases may be inserted wherever dependencies permit. Future macro/plugin/drawing releases can likewise be split when their gates demand it.

## 8. Future subsystem rules settled in discussion

### 8.1 Sketches and assemblies

Sketch constraints may be inferred from snaps with visible pre-commit hints; automatic inference can be toggled. Expose solver state as under-constrained, fully constrained, over-constrained or invalid, with conflicting relation IDs. A valid under-constrained sketch may drive an extrusion with a warning. AI creates/repairs sketches through the same typed command pipeline as UI.

Assembly definitions may be internal or pinned external-project revisions. Component instances hold transforms, state and BOM metadata. Pinning makes a build reproducible; newer external revisions are offered, not silently adopted. On a conflicting mate, reject the new relation and explain conflict. Collision checks separate broad phase from accurate narrow phase and report method/clearance. Imported STEP parts without source history remain geometry-backed components.

### 8.2 Drawings, materials and analysis

Drawing views are associative with a model revision and their displayed dimensions bind to semantic references. Model edits mark a drawing stale; rebuilding is explicit. Reference failures remain visible. Validate concrete ГОСТ/ЕСКД and ISO formats and projection rules when implementing, then record exactly which variants are supported. Do not treat visual resemblance as standard compliance.

Material records include source, units, version and applicability, with user-defined entries. A project snapshots assigned properties. A library update proposes a controlled project update, and existing simulation/mass results become stale after accepted property change. Process profiles carry machine/material/tool assumptions. General geometry warnings are available without a profile; an overall production verdict requires relevant inputs.

The first simulation workflow is linear static analysis of one part. Select a first local solver after checking licensing, available builds and reliable operation on target platforms. Additional CalculiX/Elmer/Code_Aster adapters follow the same input/result interface. A stored result includes geometry revision, mesh, material snapshot, constraints, loads, solver identity/version and logs. The AI may explain but cannot certify a physically invalid setup.

Slicer/CAM and equipment control are separate future integrations after analysis/export. Never start a physical machine as a side effect of opening a project, importing a plugin, or reading an AI message.

### 8.3 Lua and WASM extension lifecycle

`.cadmacro` and `.cadplugin` archives require bounded size, versioned manifest, hash validation, traversal-safe extraction, local installation, disabled/enabled state and audit logs. They do not gain direct filesystem, shell, OS or network access. Expose host functions through capability-specific APIs and the central Permission Manager. Validate timeout, memory/fuel/loop limits and failure isolation.

Declarative UI schemas render through Forma's localized themed component system. Plugin-authored labels remain author content but host controls, permission explanations, errors, system messages and accessibility labels are localized. UI schemas, commands, context actions, overlays and shaders have explicit validation/versioning. Validate shader resources before compilation and enforce GPU resource budgets.

Custom plugin feature nodes record plugin ID, pinned version, API version, input schema version, parameters, upstream references, generated topology mapping and cached last valid BREP. If the plugin is absent, permit view/measure/export from cached geometry while blocking rebuild of that node and dependents. A plugin update is installed side-by-side; a project upgrades only by explicit action, backup and successful staged rebuild. A package can include extension files by user choice; imported extensions stay inactive until permission review.

### 8.4 Revisions, branches and future optimization

Every successful significant operation records actor, source, affected IDs, parent, time, summary, command record and validation state. Revision restore appends a new revision. Pinned measurements, drawings, simulations and plugin nodes refer to semantic entities and expose stale/broken status after changes. Comparison includes parameter/feature semantic changes and BREP visual juxtaposition, with later exact difference maps.

Branch merge must not silently select a geometrically plausible but wrong result. Automatic merge is limited to independent changes with deterministic conflict checks. AI assistance may explain conflicts or propose a typed resolution plan; final merged geometry must rebuild from the command graph and pass validation. Optimization variants use branch/revision semantics without overwriting the chosen main model until explicitly adopted.

## 9. Quality, security, operational work

**Native quality:** CMake/Ninja builds, clang-format and targeted clang-tidy; CTest unit/integration coverage for exception boundary, primitive/feature operations, topology mapping, STEP round-trip, Boolean/fillet failure, measurements and tessellation. Rust cargo fmt/clippy/test with transaction, sandbox, process cancellation, journal and package cases. TS strict mode, lint, Vitest/RTL for command UI and permissions, Playwright for end-to-end desktop flows. Avoid mirror-image tests of trivial wiring.

**Adversarial cases:** path traversal through archives and project filenames, symlink/reparse-point escape, shared-folder race, command injection through executable/argument configuration, permission bypass, malformed STEP/BREP/mesh, oversized archive/decompression bomb, invalid JSON/IR, corrupted project and cache, worker crash, stale plan, C++ exception, malicious WASM, infinite Lua macro, invalid shader, plugin timeout/memory/fuel exhaustion, unsafe external-path access. Check actual prevention at the host boundary rather than trusting a UI disabled state.

**Data and logs:** Local activity records include request ID, project, actor, action, decision, revision and outcome, with sensitive prompt/geometry logging off by default. Rotated logs stay local. Crash reports do not upload automatically. Cache size and automatic performance tuning are configurable. Project package import must never execute bundled code.

**Release engineering:** Preserve existing npm-workspace pipeline unless a controlled pnpm migration proves equivalent or better and the Windows packaging/updater workflow remains intact. Store updater signing keys outside Git. Build the CAD worker and OCCT dependencies reproducibly for Windows x64, macOS arm64/x64, Linux; verify platform license notices, rpaths/DLL search, codesign/notarization where configured, and offline startup. A missing update server never blocks CAD work. Keep published version artifacts immutable.

**Documentation:** Maintain a public manual for shipped behavior only. Keep this private planning file out of Git. Before each release update ru/en UI keys, check both themes and languages, run `npm run check:i18n` and `npm run check:migrations`, validate new migration files keep LF, verify docs/release notes and installer artifacts. Distinguish verified functionality from future ideas.

## 10. Explicit decisions still requiring implementation evidence

These are engineering evaluations, not missing product approval:

1. Benchmark and select project metadata storage, lock protocol and file layout on local disks and network shares.
2. Measure exact operation parity against 1.1 fixture models and decide the command vocabulary needed for 2.0, including advanced geometry formerly available through CadQuery.
3. Test the OCCT redistribution/build strategy and license notices on every required platform.
4. Validate WebGPU availability in supported Tauri webviews and decide when to fall back automatically to WebGL2, including shader equivalence.
5. Define numerical tolerance and geometric regression fixtures for representative printing and precision mechanical parts.
6. Define portable handoff between installed app and external data folder, including failure and rollback handling.
7. Select the first local AI adapter and first simulation solver only after install/protocol/platform evaluation.
8. Resolve the observed 1.1.0/1.2.0 version mismatch before assigning a new version to any application change.

The product decisions above are settled unless the user revises them. The original MASTER SPEC remains a source of long-range requirements, but this document records the agreed release boundaries and behavior when the two disagree.

# Подробный план реализации на русском языке

Англоязычная часть выше содержит технические контракты и контрольные сценарии. Этот раздел раскрывает решения и последовательность работ на русском. Он является основной рабочей инструкцией для реализации; при расхождении формулировок использовать зафиксированные ответы пользователя из обсуждения.

## 11. Состав 2.0.0: что пользователь сможет сделать

### 11.1. Сквозной сценарий создания детали

1. Пользователь устанавливает Forma и запускает её без регистрации. Установщик уже содержит CAD-ядро; Python и CadQuery устанавливать не требуется. Без интернета можно создать и открыть проект, построить модель вручную, измерить и экспортировать её.
2. На первом запуске короткий мастер предлагает выбрать папку проектов, единицы, тему и найденный AI CLI. Каждый шаг можно пропустить. В дальнейшем открывается Dashboard; восстановление прежних вкладок включается отдельно.
3. Пользователь создаёт проект «Корпус контроллера», указывает миллиметры и при желании импортирует STEP платы либо прикрепляет изображение. Проект физически существует как переносимая папка; одна только запись в глобальной базе не должна быть условием его открытия.
4. Запрос «сделай корпус для этой платы» заставляет AI уточнить размеры платы, если он не может надёжно получить их из файла. Предположения о толщине стенки, зазорах и креплениях перечисляются в плане. Сохранённые требования проекта учитываются; найденное противоречие показывается до построения.
5. План содержит понятные операции, целевые размеры, затрагиваемые тела и ожидаемые проверки. Одобрение запускает одну транзакцию. Успех создаёт одну ревизию; ошибка оставляет прежнюю модель. В пределах того же плана AI может сделать две попытки исправления.
6. Пользователь выбирает крышку или конкретное ребро. Выбор подсвечивается и показывается возле поля AI. Команда «скругли эти рёбра на 2 мм» ссылается на постоянные геометрические ID. После перестроения выполняются точные измерения; неоднозначная ссылка запрашивает повторный выбор.
7. В параметрах пользователь одновременно меняет ширину и толщину. Он может нажать «Предпросмотр», а может сразу «Применить». Приложение перестраивает и проверяет модель. Всё изменение — один шаг Ctrl+Z. Нажатие Escape до применения отбрасывает черновик.
8. Пользователь сравнивает ревизии рядом либо наложением, выбирает для экспорта весь проект или отдельные тела, назначает единицы и качество сетки. При необходимости делает полный `.cadpack` или копию для передачи с выбором вложений и истории.

Проверять этот путь минимум на двух классах моделей: корпус для печати с отдельной крышкой и точная механическая деталь с отверстиями, фасками/скруглениями и импортированной STEP-геометрией. Наличие кнопки без успешного прохождения сценария не считается завершённой функцией.

### 11.2. Что входит и что отложено

| Подсистема | Обязательно работает в 2.0.0 | Следующие выпуски |
| --- | --- | --- |
| Моделирование | Шаблоны 1.x, простые параметризованные тела, существующие пользовательские геометрические операции в новом типизированном API, несколько именованных тел, импорт STEP с операциями поверх него | Формулы, полноценный ручной Sketcher, перетаскивание граней, полная история с подавлением |
| Проверка модели | Выбор тел/граней/рёбер/вершин, точные расстояния, углы, радиусы/диаметры, длина, площадь, объём, габариты, закреплённые измерения | Кривизна, толщины, производственные карты, углублённые инженерные режимы |
| AI | Codex, Claude, Custom CLI, несколько диалогов, память с подтверждением записей, выбор геометрии в контексте, план, разрешение, атомарное выполнение, проверенный отчёт | Локальный AI в раннем minor, многоролевые процессы и расширенный контекст позже |
| История | Ревизии, восстановление новой ревизией, undo/redo команд, сравнение рядом и наложением | Локальные ветки и безопасное слияние отдельным выпуском |
| Файлы | Самостоятельная папка проекта, блокировка записи, recovery, импорт/экспорт текущих форматов, `.cadpack`, portable data mode | Расширенные импортёры, макросы и плагины как отдельные пакеты |
| Отображение | WebGPU при доступности, полноценный WebGL2 fallback, сечение, X-Ray/Ghost, режимы из 1.x, автоматическое качество | Углублённые WGSL-карты и масштабирование больших сборок |
| Производство | Точная геометрия, измерения и экспорт для внешнего производства | Материалы, анализ печати/CNC, слайсеры/CAM, локальные решатели |

Полноценные чертежи, сборочные сопряжения, BOM, Lua, WASM, симуляция и оптимизация не показываются в 2.0 как пустые экраны. При этом их будущие форматы и права учитываются в дизайне IR, ревизий, хранения и Permission Manager.

## 12. Архитектура ядра и формат CAD IR

### 12.1. Границы процессов

Главный процесс Tauri хранит состояние проектов, очередь, права, транзакции, адаптеры AI и журнал событий. Отдельный локальный CAD worker содержит Rust-оболочку и C++20/OpenCascade. Связь Rust ↔ C++ реализуется через `cxx`; C++ исключения перехватываются внутри worker и превращаются в типизированные ошибки. Пользовательский интерфейс отправляет только типизированные запросы в Rust. Он не получает свободный доступ к shell, диску или сетевому API.

Для управляющих сообщений достаточно небольшого версионированного формата с `request_id`, `project_id`, ожидаемой ревизией, типом операции и результатом. Большие массивы вершин, индексов и атрибутов передаются как бинарные буферы или проверенные временные файлы с контрольной суммой, а не как огромный JSON. Worker создаёт BREP и tessellation; viewport показывает производную сетку. Кэш можно удалить без потери исходной модели.

Сначала реализовать worker с простыми телами, STEP и точным измерением. Потом добавить операции из матрицы совместимости, топологические ссылки, секущую геометрию и фоновую tessellation. Прежний Python-путь удалять из обязательной установки только после прохождения паритетных сценариев на Windows, macOS и Linux.

### 12.2. Документ, команды и ссылки

CAD IR v2 хранит тела, операции, входы операций, параметры, единицы, зависимости, состояние перестроения, происхождение импортированных объектов и семантические ссылки. У каждого объекта постоянный ID, а имя для человека меняется отдельно. Ссылка на грань или ребро не может быть номером треугольника или индексом массива после tessellation.

Команда содержит источник (`ui`, `agent`, позднее `macro`, `plugin`), действующего субъекта, ожидаемую ревизию, тип операции, постоянные ID целей, значения с единицами и идентификатор плана/транзакции. Схема проверяет допустимый тип, диапазоны, зависимости, отсутствие циклов, доступ к целям и нужные права до вызова CAD worker.

Пример концептуального плана: `set_parameter(width, 120 mm)`, `add_hole(body_id, face_ref, 6 mm, position)`, `fillet(edge_refs, 2 mm)`. Это описание контракта, а не обещание зафиксировать именно такие имена полей до проверки всех операций 1.1. Форматы документов, команд, worker IPC, настроек, плагинов и макросов версионируются отдельно. Для каждого формата нужны явные изменения схемы и тесты загрузки прежних версий 2.x.

### 12.3. Постоянное именование топологии

При создании каждой операции ядро сохраняет происхождение выходных граней и рёбер: операция-источник, входные ссылки, вид поверхности/кривой, ориентацию, соседство и устойчивые геометрические признаки. После перестроения сопоставление даёт одно из состояний: однозначно сохранена, исчезла или неоднозначна. Только первое может продолжаться автоматически. При неоднозначности UI показывает кандидатов на модели; пользователь вручную подтверждает новую ссылку в staged-транзакции. Нельзя выбирать «ближайшую грань» без проверки и затем молча менять не тот объект.

Проверочные примеры: изменение ширины корпуса без потери ссылок на исходные отверстия; добавление скругления с изменением числа граней; импорт STEP с новой операцией; изменение порядка независимых операций. Для каждой ссылки хранить тест, который доказывает либо правильное сохранение, либо явный конфликт.

### 12.4. Транзакции, ревизии и отмена

Алгоритм изменения: проверить ожидаемую ревизию → проверить права и входные данные → подготовить отдельное состояние → исполнить команды → перестроить затронутые зависимости → проверить BREP и ссылки → сохранить артефакты во временном поколении → зафиксировать единую новую ревизию → переключить указатель текущей модели. До последнего шага старая модель остаётся рабочей. Журнал восстановления различает подготовленное и подтверждённое поколение.

AI-план применяется целиком, ручная пачка параметров — целиком, один запуск будущего макроса — целиком. Ошибка любого шага откатывает CAD-изменение целиком. Внешний экспорт уже созданного файла не является частью обратимого изменения модели, поэтому его статус записывается отдельно. Undo/redo опирается на историю команд/состояний CAD IR и валидированные ревизии. Восстановление старой ревизии создаёт новую запись, сохраняя линию истории.

Каждая ревизия хранит ID, родителя, источник, автора/провайдера или плагин, время, изменённые ID, понятное описание, набор команд, версию IR и результат валидации. История сравнивает значения параметров и операции; визуальное сравнение рядом синхронизирует камеры, наложение использует разные цвета для прежней и новой геометрии.

## 13. Проект на диске, перенос и восстановление

### 13.1. Структура и выбор хранилища

Зафиксировать внешний контракт проекта: версия схемы, ID проекта, список тел и операций, граф зависимостей, стабильные ссылки, ревизии, пользовательские требования, диалоги, вложения, экспорт и служебный журнал. Внутри допускается база, файлы или гибрид. Решение принимается по измерениям: запуск проекта на SSD, открытие истории с тысячами ревизий, работа с большим числом вложений, сохранение после изменения одной операции, копирование на NAS, авария между записью тела и указателя ревизии, создание/распаковка `.cadpack`. По результатам выбрать формат и записать правила атомарного обновления.

Глобальное хранилище приложения содержит лишь то, что относится ко всему приложению: список последних/закреплённых проектов, путь к папке проектов, оформление, раскладку клавиш, обнаруженные AI-провайдеры, правила разрешений и настройки обновлений. Проект должен открываться на другом компьютере после переноса его папки без глобальной записи прежней установки. Кэш tessellation и миниатюр отделяется от данных, необходимых для воспроизводимого открытия.

Блокировка проекта включает идентификатор владельца, время обновления и путь к проекту. Второй экземпляр предлагает просмотр или сохранение копии. «Принудительное открытие для записи» не является обычной кнопкой. В общей папке проверяются изменение ревизии, конфликты файлов и возможная задержка синхронизации; один файл `project.lock` сам по себе не даёт гарантию для облачно-синхронизируемых каталогов. При пропадании соединения новые транзакции запрещаются; локально сохраняются только черновики, а не создаётся втайне расходящаяся копия проекта.

### 13.2. Версии схемы и прежние проекты

В начале открытия прочитать версию каждого формата. Если проект создан более новой несовместимой версией, открыть в режиме чтения, если сохранённая геометрия доступна; не переписывать его прежним приложением. Для перехода внутри 2.x: сделать полную проверенную резервную копию → мигрировать в отдельную папку/состояние → проверить ссылки, ревизии и контрольные суммы → атомарно заменить активную версию. При ошибке прежняя папка продолжает открываться.

Применённые SQL-миграции исходного приложения неизменны. Если новый формат использует SQL, изменения добавляются новыми файлами с LF. Перед релизом выполнить `npm run check:migrations` и отдельные миграционные сценарии. Установленная 2.0 не удаляет данные 1.1. На Dashboard есть раздел прежних проектов. Копирование из STEP создаёт новый 2.0-проект с импортным базовым телом; переписка и старый CadQuery-исходник могут сохраняться как справочные данные только при явном выборе, без обещания параметрического переноса.

### 13.3. `.cadpack`, резервные копии и portable mode

Пакет проекта содержит manifest с версией, списком файлов, размером и SHA-256. При импорте до распаковки проверяются общий размер, число файлов, глубина путей, относительные пути, контрольные суммы и наличие обязательных частей. Распаковка идёт в временную папку, затем проект валидируется и атомарно появляется в выбранном каталоге. Исполняемые компоненты внутри пакета не запускаются.

«Полный проект» включает актуальную модель, историю, диалоги, память, вложения и применимые зависимости. «Копия для передачи» позволяет исключить переписку, личные вложения, несущественные экспорты и черновики, показывая состав перед упаковкой. Если исключение ломает открытие модели, оно недоступно. Учётные данные CLI, секреты и права на системные пути никогда не включаются. Будущие плагины и макросы можно вложить по выбору; после импорта они выключены и отдельно требуют просмотра разрешений.

Перед миграцией копия обязательна. Плановые копии пользователь настраивает по месту, частоте и глубине хранения; каждая копия проверяется на читаемость и не перезаписывает последнюю рабочую копию при неудаче. После crash открывается последняя подтверждённая ревизия, показываются прерванные операции и предлагается восстановление черновиков; AI-запросы не запускаются повторно автоматически.

Обычная установка может переключиться на выбранный каталог данных, например на USB или внешнем SSD. Перенос сначала копирует и проверяет данные, затем меняет путь следующего запуска; ошибка сохраняет прежний рабочий путь. Отдельная portable-сборка позволяет запустить программу на другом компьютере подходящей платформы без установки. Если носитель доступен только для чтения, Forma сообщает об этом до открытия проекта для записи.

## 14. AI, разрешения и проверка результата

### 14.1. Адаптеры провайдеров

В 2.0 поддерживаются Codex, Claude и настраиваемый Custom CLI. У каждого адаптера одинаковый жизненный цикл: обнаружить исполняемый файл и версию, сообщить возможности, создать сеанс с ID, передать ограниченный контекст, получать нормализованные события, отменить запрос, получить код выхода и диагностические сведения. События включают запуск, сообщение, план, запрос инструмента, прогресс, запрос разрешения, предупреждение, ошибку, завершение и отмену. UI не разбирает сырой поток конкретной программы как источник истины.

Custom CLI настраивается как путь к исполняемому файлу и массив аргументов, с контролируемой рабочей папкой, тайм-аутом и обязательной JSON-схемой. Сначала достаточно базового входа/выхода; дополнительные вызовы инструментов Forma подключаются, если адаптер объявил такую возможность. Ни один аргумент не собирается в shell-строку. Отправка изображения допустима только при поддержке выбранным адаптером.

Первый локальный AI-провайдер появляется в раннем 2.x и пользуется той же схемой плана, проверкой команд и правами. Отсутствие локального AI в 2.0 не меняет требования офлайн-работы самого CAD-приложения: локальное создание, редактирование, измерение, импорт, экспорт и восстановление доступны без сети.

### 14.2. Контекст, память и приватность

Контекст включает текущую ревизию, единицы, дерево операций, именованные размеры, выбранные semantic IDs, доступные проверенные измерения, требования проекта и краткую историю нужного диалога. Для длинного разговора делается краткое резюме, но исходные сообщения локально сохраняются. AI может предложить добавить решение в память проекта; оно становится постоянным только после подтверждения пользователя. Требование «не двигать эти отверстия» нельзя заменить очередным предположением AI.

В проекте может быть несколько именованных диалогов. Все они работают с одной текущей моделью. План, составленный на старой ревизии, не применяется без повторной проверки. При переключении провайдера пользователь видит, какие сведения передаются новому провайдеру; авторство прежних сообщений сохраняется. Файлы отправляются во внешний AI только как явно прикреплённые вложения после проверки формата. Приложение не отправляет целый проект автоматически.

Если пользователь выделил грань, возле поля сообщения показан тип и краткое описание выделения. AI получает точную геометрическую ссылку и проверенные данные, а не экранные координаты клика. Если после перестроения прежняя ссылка перестала быть однозначной, AI не продолжает изменение до разрешения конфликта.

### 14.3. Предпросмотр плана и исполнение

Перед запуском показать: цель, шаги, тела/операции, исходные и целевые размеры, выбранные ссылки, допущения, затронутые требования и ожидаемые проверки. Геометрический 3D-предпросмотр до подтверждения не обязателен в 2.0. У пользователя есть подтвердить, отклонить или попросить изменить план. Изменённый план не использует старое одобрение.

После одобрения агент может подготовить только разрешённые типизированные команды. Validator проверяет командный набор, Permission Manager — право изменять конкретный проект и внешние действия. CAD worker строит staged-результат. Проверка включает корректность BREP, непустые тела, допустимые размеры, успешность булевых операций, топологические ссылки и выбранные измерения. Если AI сказал «отверстие 8 мм», отметка «проверено» появляется только при независимом измерении ядром. Непроверяемые требования перечисляются как непроверенные.

При неудаче AI получает структурированный код ошибки и может предложить до двух исправлений, не меняющих смысл одобренного плана. Каждая попытка стартует с последней подтверждённой ревизии. Если исправление требует новых размеров, удаления тела либо изменения согласованной цели, нужен новый план пользователя. После исчерпания попыток показываются ошибка и предлагаемые действия; проект остаётся на прежней ревизии.

### 14.4. Разрешения и безопасность выполнения

Одобрение плана разрешает описанные CAD-изменения модели. Запуск внешней программы, запись за пределами проекта, сеть, импорт расширения и другие чувствительные операции требуют отдельного разрешения. В диалоге показываются источник действия, исполняемый файл, аргументы, рабочая папка, затрагиваемые пути и причина. Решения: один раз, на сеанс, для проекта или запретить. Опасные права не предлагаются как постоянные по умолчанию. Разрешения можно просматривать и отзывать; каждое решение попадает в локальный журнал.

Песочница применяет канонические проверки путей, запрет `../`, проверку symlink/reparse-point и явное согласование внешнего пути. AI-процесс получает ограниченные инструменты Forma для чтения, измерений и подготовки команд. Доступ к диску, shell и сети нельзя считать запрещённым лишь потому, что AI получил текстовую инструкцию «не пользоваться ими»: ограничения обеспечиваются запускающим процессом и host API на каждой целевой платформе.

## 15. Интерфейс, viewport и измерения

### 15.1. Основные экраны

Dashboard показывает недавние и закреплённые проекты, создание из шаблона, открытие локальной папки, импорт CAD и `.cadpack`, поиск, последние экспорты, раздел проектов прежней версии. В карточке проекта видны путь и дата последнего изменения; недоступный путь не удаляет карточку автоматически. Первый запуск предлагает короткий пропускаемый мастер, последующие запуски открывают Dashboard.

Рабочее окно строится вокруг 3D viewport. Сверху расположены вкладки открытых проектов и компактные группы инструментов. Слева отдельные вкладки «Модель», «Файлы», «Параметры». Справа — «AI» и «Свойства». Снизу по запросу открывается «История», «Задачи», «Активность» или «Логи». Строка состояния показывает текущий проект, единицы, режим выбора, задачу и ошибки. Размеры и видимость зон сохраняются для проекта; переключение между вкладками проектов не сбрасывает камеру, выделение или черновик сообщения.

Дерево модели и viewport связаны двусторонне. Для тела показываются имя, видимость и тип геометрии; для операции — имя, входные зависимости, числовые параметры, результат перестроения и ошибка. Переименование не меняет ID. Контекстные действия включают доступные на данном этапе команды. Выделение при открытой вкладке AI не переключает автоматически на «Свойства»; возле сообщения появляется явная метка выбранной геометрии.

Палитра команд ищет команды, настройки и объекты открытого проекта, разделяя результаты по категориям. Горячие клавиши редактируются через список с обнаружением конфликтов, поиском и сбросом. Когда курсор находится в поле текста, сочетания редактора не запускают CAD-команды. В плотном интерфейсе все значки имеют локализованные подсказки и доступные подписи для клавиатуры и экранного диктора.

Оформление нейтральное: сдержанные поверхности, тонкие границы и небольшие скругления, без декоративных градиентов. Акцент пользователь выбирает в настройках; цвета ошибок, предупреждений и выделения независимы от него. Русский используется по умолчанию, английский доступен, выбранный язык сохраняется. Стандартная плотность включена по умолчанию, доступны компактная и крупная; масштаб шрифта отдельный. Светлая и тёмная темы поддерживают весь viewport, в том числе фон и сетку, и переключаются без потери сцены.

### 15.2. Выбор и измерения

При наведении viewport подсвечивает возможный объект. Автоматический выбор дополняется фильтрами «Тела / Грани / Рёбра / Вершины», чтобы попадать в маленькие элементы. Множественное выделение позволяет измерять между объектами и формулировать AI-запросы вроде «скругли эти четыре ребра». Камера по умолчанию вращается перетаскиванием левой кнопки на пустом месте; клик по телу выделяет. Правая кнопка открывает контекстное меню, её перетаскивание перемещает камеру, колесо меняет масштаб. Управление перенастраивается.

Измерения ядром: расстояние между точками/объектами, длина ребра, радиус, диаметр, угол, площадь поверхности, объём и габариты. Результат показывается возле модели и в свойствах с единицами проекта. Его можно закрепить; после изменения модели закреплённое измерение пересчитывается. Если ссылка перестала существовать или стала неоднозначной, измерение получает явную ошибку. Закрепление измерения не создаёт геометрического ограничения. Для STL/OBJ/GLB результат помечается «по сетке» и не приравнивается к точному BREP-измерению.

### 15.3. Рендеринг и анализ вида

На доступном оборудовании сначала проверять WebGPU. При отсутствии поддержки или ошибке инициализации автоматически переходить на WebGL 2. В обоих путях должны работать загрузка, выбор, основные режимы, измерения и сечение. Список режимов 2.0: затенение, затенение с рёбрами, каркас, прозрачность, X-Ray и Ghost. X-Ray показывает внутренние элементы через поверхность; Ghost визуально приглушает окружение выбранного тела. Видеокарта может менять качество tessellation viewport, но не BREP, значения параметров и экспортную геометрию.

Сечение задаётся XY/XZ/YZ либо выбранной плоской гранью. Плоскость можно смещать числом/перетаскиванием и свободно вращать с числовыми углами. Во время движения работает быстрое графическое отсечение. По отдельной кнопке C++-ядро вычисляет точный контур пересечения, который можно измерять. После движения плоскости или изменения модели старый точный контур становится устаревшим до нового расчёта.

Настройка «Производительность → Автоматически» оценивает доступные ресурсы и текущую скорость отображения. Во время движения камеры временно снижает детализацию и эффекты, после остановки восстанавливает качество. В настройках видны выбранные значения и ручные переключатели. На экранах всех размеров сохраняется возможность точно выделить ребро; понижение детализации не должно менять semantic ID выбранного объекта.

### 15.4. История и задачи

Нижняя панель обычно закрыта. В статусной строке видны прогресс и число ожидающих CAD-задач; нажатие открывает нужную вкладку. На одном компьютере в 2.0 выполняется одна тяжёлая CAD-задача одновременно. Запланированные задачи можно отменить до запуска. При выполнении сначала посылается мягкий запрос отмены; если worker не отвечает, он завершается, staged-файлы удаляются после проверки и новый worker запускается для следующего задания. AI-запросы планирования идут отдельно и не занимают CAD-очередь.

После сбоя приложение показывает последнюю корректную ревизию, прерванную задачу и найденные черновики. Кнопка восстановления черновика не означает повторного исполнения AI-плана. В активности видны изменения модели, экспорты, импорты, ошибки и решения по разрешениям; технические логи открываются отдельно.

## 16. Пошаговая программа работ для 2.0.0

Каждый этап имеет результат, который можно показать и проверить. Работа идёт в ветке разработки; завершение этапа не означает выпуска версии. Сборка нового установщика и повышение версии обязательны при завершённых изменениях приложения по `AGENTS.md`; этот документ не меняет код приложения.

### Этап А. Зафиксировать нынешнее поведение

Собрать не только список методов CadQuery, но и реальные пользовательские сценарии 1.1.0. Зафиксировать шаблоны, пять типизированных операций, доступные через совместимый CadQuery-путь операции, импорт STEP, экспорт форматов, существующие измерения/сравнения и работу AI. Для каждого сценария сделать входную модель и ожидаемые свойства: число тел, объём, габариты, радиусы/отверстия, корректность STEP после повторного импорта. Измерить текущие времена работы как базу. Условие завершения — матрица «сценарий → команда нового IR → результат → тест» без молчаливых пробелов.

### Этап Б. Контракты данных и версии

Описать IR v2, Command API, модель прав, версии проектов и worker-протокола. Составить примеры корпуса с крышкой, механической пластины, импортной STEP-детали и тела с несколькими зависимыми операциями. Проверить, что схема умеет представить их без неструктурированного исходного кода. Типы Rust/TS/C++ связываются генерацией либо проверенными адаптерами, а изменения схем проходят совместимые тесты. Условие завершения — одинаковое чтение/запись тестовых документов на всех границах и явный отказ неизвестным модифицирующим командам.

### Этап В. CAD worker и C++/OCCT

Собрать отдельный процесс на Windows, macOS arm64/x64 и Linux, включить нужные библиотеки в пакет, проверить юридические уведомления и пути загрузки DLL/dylib/so. Реализовать тестовый обмен, создание тел, STEP import/export, tessellation и ошибку C++ без падения главного процесса. Добавить лимиты памяти/времени, отмену и перезапуск. Условие завершения — worker строит тестовые тела офлайн на всех платформах, а его авария не уничтожает проект.

### Этап Г. Геометрический паритет

Переносить операции по группам: примитивы и профили, выдавливание и вращение, трансформации, булевы операции, отверстия/массивы, скругление/фаска, оболочка, sweep/loft и остальные реальные операции нынешних сценариев. Для каждой группы: положительные и ошибочные размеры, проверка BREP, сопоставление ссылок, независимые измерения, STEP round-trip, обновление viewport. Если какое-то действующее в 1.1 пользовательское построение невозможно, 2.0 нельзя считать достигшим согласованного паритета; нужно добавить типизированную операцию или согласовать изменённое поведение.

### Этап Д. Проект, история и надёжность

Измерить кандидаты хранилища, утвердить проектную папку и `.cadpack`, реализовать блокировку и режим чтения, атомарные staged-транзакции, журнал восстановления, autosave, ревизии, undo/redo, резервные копии и миграции 2.x. Прогнать отключение питания/убийство процессов в разных местах сохранения. Условие завершения — после каждого сбоя открывается целая последняя подтверждённая модель либо проверенная резервная копия; частично сохранённой модели нет.

### Этап Е. AI и Permission Manager

Общий интерфейс адаптеров, структурированные события, Custom CLI, подготовка контекста, выбор граней/рёбер, несколько диалогов, память, предварительный план, разрешения, два ограниченных исправления, верифицированный отчёт. Условие завершения — AI не может изменить модель вне одобренной транзакции, старый план не применяется поверх новой ревизии, неподдерживаемый CAD-запрос не запускает произвольный код.

### Этап Ж. Viewer и интерфейс

Новый Dashboard, вкладки проектов, зоны рабочего пространства, палитра, клавиши, темы/локализация. Viewer получает semantic selection, точные измерения, сечение и оба графических пути. Проверить сохранение камеры, выделения, черновика и layout при переключении проекта и темы. Условие завершения — весь сценарий из §11 проходит как на WebGPU, так и через WebGL2 fallback.

### Этап З. Релизная проверка

Прогнать безопасность путей, пакетов, запуска CLI и CAD worker; проверить откат, резервные копии и смену версии. Собрать локальные Windows EXE/установщик и пакеты macOS/Linux в CI, проверить наличие артефактов и работу офлайн. Синхронизировать npm workspace, lockfile, Cargo и Tauri, привести версии к 2.0.0 только при фактически завершённом объёме. Обновить ru/en строки, пользовательскую документацию и release notes. Публикация в интернете и установка на компьютер пользователя требуют отдельного указания.

## 17. Детальный план линии 2.x после архитектурного выпуска

Номера ниже — рабочее расписание зависимостей, а не обязательство уложить крупную функцию в один релиз. Если критерий завершения не проходит, этап дробится на дополнительные minor-версии. Уже выпущенный номер не переиспользуется. Для каждой новой схемы проекта, CAD IR, макроса, плагина, настройки и ревизии предусматриваются совместимая загрузка и миграции.

| Этап | Пользовательский результат | Необходимые детали и критерий готовности |
| --- | --- | --- |
| 2.1 — Параметрическое ядро | Формулы вроде `inner_width = width - 2 * wall`, зависимые размеры, единицы и перестроение только затронутой части | Парсер выражений без исполнения кода, стабильные ID, граф зависимостей, ошибки единиц и циклов, понятный список причин перестроения, undo/redo |
| 2.2 — Первый локальный AI | Моделирование через один проверенный локальный провайдер без интернета | Обнаружение и настройка локального адреса/процесса, показ ограничений модели, тот же JSON-план, права, выбранная геометрия и проверка результата, что у CLI |
| 2.3 — Полная история операций | Подавление, включение, зависимости, частичное перестроение и состояния ошибок | Корректная перестройка после изменения ранней операции, отображение затронутых потомков, конфликт ссылок, безопасное удаление/переупорядочивание там, где это допустимо |
| 2.4 — Ручной Sketcher | Построение линий, дуг, окружностей, сплайнов и вспомогательной геометрии на плоскости | Стабильные ID элементов, сетка/привязки, trim/extend/offset/mirror, видимые предлагаемые автоматические связи |
| 2.5 — Связи эскизов | Размеры и геометрические ограничения с ясным статусом эскиза | Заменяемый solver, диагностика конфликтующих связей, допустимость недоопределённого корректного эскиза с предупреждением, запрет ошибочного построения |
| 2.6 — Прямое редактирование | Выбор грани/ребра, перетаскивание мышью и точное число в одном инструменте | Быстрый staged-предпросмотр, подтверждение/Escape, операция появляется в истории, потеря ссылки вызывает конфликт, AI и UI используют одни команды |
| 2.7 — Основы сборок | Дерево деталей, внутренние части, экземпляры и внешние проекты с закреплённой ревизией | Изоляция, скрытие, перемещение, взрыв-схема; общая деталь для нескольких экземпляров; пакет включает необходимые зависимости |
| 2.8 — Сопряжения и BOM | Закрепление, соосность, совпадение, расстояние/угол, проверки пересечений и ведомость деталей | Solver показывает оставшиеся степени свободы, новая конфликтующая связь отклоняется, BOM экспортируется с проверенными метаданными |
| 2.9 — Инженерный viewer | Дополнительные режимы выбора и карты нормалей, кривизны, толщины | У каждой карты известен метод расчёта; взаимодействие остаётся доступным при WebGL2 fallback; визуальный цвет не объявляется инженерной величиной без легенды |
| 2.10 — Чертежи | Ассоциативные виды, размеры, сечения, допуски, основная надпись, PDF/SVG/DXF | Начать с проверенных вариантов ГОСТ/ЕСКД и ISO; после изменения модели чертёж устаревает и обновляется по действию пользователя; потерянный размер отмечается |
| 2.11 — Lua | Импорт/экспорт `.cadmacro`, локальный редактор, запуск, настройки и запись действий | Lua 5.4 без прямого доступа к ОС; команда проходит через IR; один запуск — одна CAD-транзакция; лимиты, остановка и логи |
| 2.12 — WASM | Локальные `.cadplugin`, версии, права, команды и собственные типы параметрических операций | Wasmtime sandbox, лимиты ресурсов, версионированный API, закрепление версии в проекте, сохранение последней геометрии при отсутствии плагина |
| 2.13 — UI/SDK плагинов | Декларативные панели и настройки, контекстные действия, overlays и SDK-примеры | Host сам рисует безопасный UI в двух темах и с доступностью; ресурсы WGSL валидируются; API имеет совместимость/миграцию |
| 2.14 — Материалы и производство | Локальная библиотека материалов, собственные записи и профили процессов | Свойства с единицами/источником, снимок в проекте, отдельный приём обновлённого материала, общие предупреждения без профиля |
| 2.15 — Анализ печати | Стенки, нависания, ориентация, опоры, габариты стола, масса и оценка времени | Заключения привязаны к профилю материала/принтера; оценка времени обозначена как приближённая |
| 2.16 — Анализ CNC | Доступность фрезы, внутренние углы, глубина карманов и вылет | Инструмент/станок в профиле, конкретные проблемные поверхности, объяснение неизвестных или непроверяемых случаев |
| 2.17 — Локальная симуляция | Первая законченная линейная статическая задача одной детали | Один проверенный solver, сетка, закрепления, нагрузка/давление, напряжение, перемещение; результат связан с ревизией и параметрами расчёта |
| 2.18 — Расширенный AI-контекст | Скриншот viewport, выбранная геометрия, вложения, краткие резюме и решения проекта | Контроль передаваемых данных, отсутствие бесконечной пересылки всей истории, сохранение авторства и версий |
| 2.19 — Роли AI | Последовательные Design/Validation/Manufacturing/Review | Провайдеры назначаются на роли, детерминированные проверки выполняет Forma, смена внешнего адресата видима |
| 2.20 — Design Review | Проверка геометрии, толщин, крепежа, материала, пересечений и изготовления | Каждый вывод маркирован: проверено, оценено или неизвестно; исправление оформляется одобренным планом |
| 2.21 — Оптимизация | Цели, ограничения, ограниченный поиск и сравнение вариантов | Число попыток/время ограничены, варианты сохраняются отдельно, основной вариант меняется только по выбору пользователя |
| 2.22 — Большие сборки | LOD, instancing, частичная загрузка, геометрический кэш и отзывчивый viewport | Реальные стендовые сборки, контроль памяти/времени открытия/кадровой частоты на разных ПК |
| 2.23 — Усиление безопасности | Устойчивость к повреждённым проектам, пакетам, WASM, Lua, CAD и небезопасным путям | Набор вредоносных фикстур, тесты на всех ОС и проверка каждой границы прав |
| 2.24 — Доводка | Восстановление, миграции, доступность, установщики и обновления | Полный регрессионный прогон линии 2.x, ясная документация и стабильные offline-сценарии |

Ветки добавляются отдельным совместимым minor-выпуском после стабилизации формата ревизий: локальное создание ветки, переход, сравнение и слияние только независимых параметров/операций/метаданных. Конфликт решается вручную; после слияния выполняется полная перестройка. Другие локальные AI-провайдеры, решатели, слайсеры и CAM-адаптеры могут занимать дополнительные номера 2.x. Цифры после 2.9 продолжаются как 2.10, 2.11 и далее.

### Особые правила будущих подсистем

**Плагины.** Поскольку собственные типы операций нужны уже в первом WASM-выпуске, нельзя строить систему плагинов только как набор кнопок UI. Документ сохраняет ID плагина, закреплённую версию, схему параметров, входные зависимости и последнюю корректную геометрию. Если плагина нет, тело доступно для просмотра, измерения и экспорта, но его операция и зависимое перестроение заблокированы. Несколько версий плагина могут сосуществовать. Обновление проекта выполняется только после резервной копии, миграции параметров и проверки сборки.

**Макросы.** Пакет `.cadmacro` содержит manifest, Lua-код и необязательные ресурсы. Встроенный редактор показывает диагностику, autocomplete host API, запуск/остановку и лог. UI настроек строится по схеме; пользователь не обязан редактировать Lua. Запись действий формирует поддерживаемые команды, а не воспроизводит произвольные клики по экрану. Ошибка макроса откатывает его CAD-транзакцию и не завершает приложение.

**Сборки.** Внешняя деталь связывается с конкретной ревизией другого локального проекта; новая ревизия не подставляется автоматически. Несколько экземпляров используют одно определение детали. Перемещение экземпляра не переписывает её геометрию. После ввода конфликтующего сопряжения прежнее правильное положение сохраняется.

**Материалы и симуляция.** Проект сохраняет точные свойства выбранного материала с источником. Первая симуляция ограничена линейной статикой одной детали. Результат содержит версию геометрии, сетку, материал, нагрузки, закрепления и solver. Изменение любого входа помечает карту напряжений устаревшей. AI может объяснить расчёт, но его словесное заключение не заменяет проверку исходных данных и локального решателя.

**Изготовление.** Анализ 3D-печати и CNC работает от выбранных профилей процесса. Без профиля допустимы общие замечания, но итог «готово к производству» не выдаётся. Генерация G-code, CAM и отправка задания на оборудование — отдельные будущие интеграции с явным подтверждением запуска.
