# Shared model request preparation / Общая подготовка CAD-запроса

Inspected base: `79431c23d59b58609248ec57df763d71031c6356`.
Patch: [storage-model-request.patch](storage-model-request.patch).
Status: **applied after lovlyDev explicitly authorized all five paths**. Existing target paths are unchanged from that base. Their latest shared extraction is approved execution commit `4a36824`. All proposed edits exist only in `.local/storage-model-request-preparation` copies and this patch.

## RU

Это точный slice C1 этапа 4: единая подготовка намерения моделирования из уже захваченного Project. Он не реализует folder apply, новые права или законченный CAD-сценарий внешней папки. Общие production-файлы при подготовке не редактировались. ApplyRequest и существующий IPC остаются прежними.

Новый `modeling/prepared_request.rs` принимает `&Project` и `&ApplyRequest`. Перенесённый единственный алгоритм сохраняет порядок: пустота/лимиты program и prompt → `program::prepare` → expected revision → ReviewPlan с текущим документом → no-op → существующий next_revision либо UUID → `commit::program_for_revision`. Тексты ошибок и байтовые лимиты 60000/16000 остаются прежними. No-op сравнивает сохранённую строку с trim входного program, как раньше. Новая проверка project_id не добавляется: host обязан сам связать request с captured source.

Результат — `PreparedRequest::NoOp` либо owned `PreparedApply` в Box с закрытыми полями PreparedProgram, Option<Document>, job и saved_program. ReviewPlan candidate сохраняет requested revision; только saved_program связывается с job через прежний `commit::program_for_revision`. Значение не сериализуется и не является grant, lease, source authority или commit receipt.

Подготовка не обращается к AppState, SQL, файлам, часам, permissions или worker. Прежняя генерация UUID остаётся единственным недетерминированным шагом после проверок/no-op. Приватный FnOnce seam позволяет доказать тестом, что ранние отказы/no-op не выбирают job. Это подготовка без backend IO; генератор UUID не объявляется математически чистым.

Managed apply сохраняет внешнюю последовательность ensure_write → projects::get → общая подготовка → ранний no-op → permissions::consume → tasks → очередь/исходные rechecks → прежний build → host write lock/recheck → прежний commit. No-op по-прежнему не потребляет grant и не регистрирует задачу. Job/saved_program по-прежнему выбираются до consume. Permission/task cleanup, queue, current_snapshot, worker и persistence не меняются. Эти свойства видны в diff; pure tests не выдаются за исполненный permission integration test.

`commands::prepare_for_snapshot(&Project, Vec<Command>, String, String)` получает captured Project и вызывает прежний единственный `cad_ir::apply_commands`. Проверки count/current revision/program/document revision, ошибки и связь next_revision сохраняются. Managed wrapper сначала вызывает общий count helper до projects::get, затем делегирует pure entry. Pure entry проверяет count тем же helper для самостоятельного вызова. Это сохраняет приоритет ошибки oversized batch перед ошибкой загрузки проекта без второго command engine.

`prepared_change.rs`, `commit.rs` и `program.rs` не входят в патч. Видимость существующего output preparer остаётся pub(super). Обёртки prepare_change и будущие неиспользуемые accessor не добавляются; suppressions dead_code отсутствуют. C2 orchestration внутри modeling сможет обращаться к уже существующему output preparer без нового export.

### Точные пять общих путей для согласования

1. `apps/desktop/src-tauri/src/modeling.rs` — регистрация crate-visible prepared_request module.
2. `apps/desktop/src-tauri/src/modeling/apply.rs` — делегация подготовки; прежний managed host/commit сохраняется.
3. `apps/desktop/src-tauri/src/modeling/commands.rs` — snapshot entry и managed wrapper с прежним count-before-load порядком.
4. `apps/desktop/src-tauri/src/modeling/prepared_request.rs` — единственная подготовка request/no-op/review/job/saved program.
5. `apps/desktop/src-tauri/src/modeling/prepared_request/tests.rs` — восемь synthetic unit-тестов.

Ни permissions/tasks/projects/native, Cargo, протокол worker, C++, UI, версии или SQL не изменяются. UUID внешней папки не превращается в managed permission key.

### Проверки и ограничения

Восемь подготовленных тестов проверяют: приоритет invalid limits/program перед stale revision; точные допустимые лимиты и однократный job sampling; no-op без sampling и изменения исходника; explicit next job и existing saved-program binding; ReviewPlan, принудительное исполнение вместо no-op и owned candidate против saved revision (native-worker-client); прежний/неизвестный body в ReviewPlan (native-worker-client); равенство command result единственному engine и неизменность исходника; count/stale/missing CAD IR/document-revision/no-change/empty-command ошибки.

Только копии отформатированы rustfmt. `git apply --check docs/integration/storage-model-request.patch` проходит на текущем worktree. Сборка, Clippy, unit-тесты и managed/native host suites при подготовке не запускались. Наличие тестов не является PASS. После отдельного разрешения main применяет ровно эти пять путей и проверяет актуальный head действующими suites. Документ не разрешает публикацию, слияние или релиз.

C2 ещё должен связать owned intent с настоящим folder snapshot/opening/location, явным host authorization, task lifetime, bounded CAS reader, shared execution и guarded writer/commit UUID resolution. Fresh-process real CAD edit/undo/redo, imported STEP и app-index deletion остаются обязательными отдельными проверками. C1 не объявляет их завершёнными.

## EN

This is prepared slice C1, not applied production code or completed external-folder CAD. It extracts existing request preparation exactly once into a snapshot-only module. Ordering remains limits → program preparation → expected revision → ReviewPlan → no-op → job selection → existing saved-program revision binding. No-op still returns before permission consumption/task registration; job selection remains before permission consumption. Existing permissions, queue, task cleanup, source rechecks, actual worker and managed commit remain unchanged.

PreparedRequest distinguishes NoOp from an owned boxed PreparedApply. Private fields retain PreparedProgram, owned ReviewPlan candidate, job and saved_program. The value carries no source/session identity, permission or publication authority. Candidate review retains the requested revision while saved_program is rebound through unchanged commit::program_for_revision. Preparation performs no backend/filesystem/clock/worker IO; existing UUID sampling remains the sole nondeterministic step after validation/no-op and has a private deterministic test seam.

commands::prepare_for_snapshot uses cad_ir::apply_commands as the sole engine. The managed adapter retains count rejection before project loading, using the same count helper that protects independent snapshot callers. Current project/document revision checks, error strings and command-generated next_revision binding remain intact.

Only the five paths above are proposed. prepared_change, commit, program, permissions, tasks, projects, native, Cargo, worker protocol, C++, SQL, UI and versions are unchanged. ApplyRequest's public API is unchanged. No unused future wrappers or dead-code suppressions are introduced; later C2 orchestration inside modeling can already call the existing pub(super) output preparer.

Only local copies were formatted and git apply --check passed; no build, Clippy or tests ran. Eight synthetic tests cover limits/error precedence, no-op/job sampling, revision binding, ReviewPlan and command behavior; two require native-worker-client. They do not prove geometry, real permission handling, durable folder publication or crash safety. After explicit approval main must apply and validate this exact patch with current CI/host suites.

C2 still needs genuine original-source/opening/location guards, folder-specific host authorization, task lifetime, bounded captured CAS reading, shared execution, guarded publication and indeterminate UUID resolution. Real fresh-process edit/undo/redo/imported STEP/app-index deletion acceptance remains outstanding.
