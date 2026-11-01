# C2: shared source-aware modeling orchestration / Общий запуск CAD

Status: applied after lovlyDev explicitly approved all six paths on 2 November 2026. Formatting and patch applicability passed; compilation and runtime verification remain required.
Follow-up: [explicit runtime constructor](storage-runtime-services.md), separately approved and applied.
Parent-supplied base: `1c5d59e`, with approved C1 `cba3824` already applied.
The patch was generated from current file copies; exact base-file SHA-256 values
are recorded in `.local/c2-repository-apply/base-sha256.txt`. The six proposed
copies were formatted with local Rust 1.97.1 rustfmt, edition 2021 and
`skip_children=true`, one file at a time. Baseline/proposed comparison texts
were normalized to LF for minimal hunks; original baseline bytes and hashes
remain intact. Read-only `git apply --check` passed against the task tree.
No compilation, unit test or worker was run.

## RU

[Точный патч](storage-repository-apply.patch) заменяет managed `apply_inner`
единственным `repository_apply::apply`. Общий алгоритм выполняет подготовку
запроса, ReviewPlan/no-op, разрешение, регистрацию задачи, очередь, отмену,
исполнение, получение совместимых последних metadata, единственную сборку
`PreparedModelChange` и вызов настоящего backend publication.

Патч меняет ровно три существующих файла: `modeling.rs`, `modeling/apply.rs`,
`modeling/commit.rs`. Добавляет `modeling/repository_apply.rs`,
`modeling/repository_apply/managed.rs`, `modeling/repository_apply/tests.rs`.
Новые модули разделяют orchestration, managed adapter и проверки. Файлы native,
permissions, core, Cargo, версии, IPC и UI не затрагиваются. `build.rs`,
`execution.rs` и C1 request preparation также остаются неизменными.

`ManagedHost` удерживает настоящий `WriteGuard` до конца операции и захватывает
`legacy::capture_with_history`, сохраняющий оригинальный `ReadSession`. После
очереди и под `state.writes` он повторно проверяет оригинальный lifecycle,
захватывает последний снимок и сравнивает `current_revision`, `files`,
`revisions` и history archive. Имена, сообщения и прочие совместимые metadata
берутся из последнего снимка. Перед persist проверяются original lifecycle и
точный latest snapshot. Managed authorization остаётся существующим
`permissions::consume`; обработка managed asset ошибок остаётся существующим
`build::build` wrapper. `commit.rs` только публикует общий prepared candidate.

Это согласуемое усиление поведения: прежняя проверка сравнивала только head и
files и загружала project через `projects::get`. Новый маршрут также проверяет
revision contents, history и original lifecycle, отказывает при pending recovery
вместо вызова reconcile. History archive обнаруживает промежуточные переходы,
даже когда текущий model head снова совпал. Metadata-only сохранение разрешено
при неизменных model/history seals. Перед применением этого патча владелец
принимает именно это изменение managed поведения; это не механический rename.

`ApplyHost` использует RPITIT и статическую dispatch, все futures — `Send`.
Associated types задают backend snapshot, access, authorization, publication
guard и receipt. `ApplyRuntime` содержит только ссылки на общий task map и
`CadTaskGate`; он не открывает проект из app SQL. `GeneratedModel` переэкспортирован
из `modeling` для folder adapter. `PreparedApply` уже предоставляет доступ к
prepared program и review document; отдельный экспорт `PreparedProgram` не
нужен для вызова существующего executor с полученным значением.

Backend receipt намеренно не унифицирован с успехом. Общий результат
`RepositoryApply::Published(receipt)` означает только возврат backend publication.
Managed receipt — подтверждённый `Project`. Будущий folder receipt должен быть
настоящим `CommitOutcome`: только `Committed` позволяет вернуть success/apply
в UI; `Indeterminate` сохраняет известный commit UUID для resolve на исходном
opening и не запускает CAD повторно. Новый common enum не теряет этот UUID.

Контракт folder adapter для следующего owned патча:

1. Host владеет или заимствует реальный `FolderSession`; snapshot и Rust-only
   authorization привязаны к original opening. Для запуска в отдельной задаче
   host владеет `Arc<FolderSession>`, paths и grant; borrow создаётся внутри future.
2. `capture` возвращает original snapshot и access/lifecycle capability. `authorize`
   проверяет typed explicit host grant, не managed permissions по folder UUID.
3. `task_key` — отдельный UUID данного opening, а не project UUID из файла.
4. `latest` вызывает owned session `latest_compatible` и сохраняет оригинальную
   opening/location authority. Оно вызывается после очереди и после lock.
5. `build` вызывает существующий `execution::build` с trusted roots и source-bound
   `AssetSource`. При async чтении CAS заранее ограниченно загружает нужные
   captured descriptors; нельзя блокировать executor через `block_on`.
6. `lock_publication` может вернуть owned guard или `()` при внутренней атомарной
   lease/CAS/SQL операции. Тогда `publish` обязано повторно сверить original
   source и latest exact snapshot внутри настоящего session commit. Публикация
   должна отвергать stale latest после common candidate preparation.
7. `publish` получает original, latest, typed authorization и единственный
   `PreparedModelChange`; извлекает candidate/pending и вызывает настоящий
   `FolderSession::commit`. Не создаёт app project rows или fake `AppState`.

Common task cleanup сохраняет прежний ограниченный контракт: все обычные Result
пути удаляют только собственную `Arc` регистрацию. Abort/panic cleanup не
заявлен. Gate permit имеет существующий RAII cleanup; detached folder publication
должна самостоятельно удерживать lease/IO authority до фактического завершения.

Предложены четыре проверки с реальными isolated managed SQL и ReadSession:
no-op без grant/task/audit и compile-time Send future; сохранение latest metadata
с отказом на same-head revision tampering; отказ после history/head ABA; stale
queued request до worker с cleanup регистрации. Последняя блокирует реальный
CadTaskGate permit и меняет head до освобождения очереди. Они не доказывают
валидность геометрии и пока не запускались.

Шесть proposed copies отформатированы rustfmt 1.97.1 с `skip_children=true`.
Read-only `git apply --check` успешен; это не проверка компиляции. После
согласованного применения требуются strict Clippy и эти tests,
существующие managed native-host/metadata-only regression tests, затем настоящий
folder worker vertical: authored model и imported STEP, fresh-process reopen/edit,
undo/redo и отсутствие app-index authority. Этот patch сам не реализует folder
adapter, production routing или весь stage 4. Production registration и выпуск
следуют только после проверки полного маршрута, по отдельному этапу интеграции.

## EN

This exact unapplied patch replaces managed `apply_inner` with one statically
dispatched `Send` orchestration. The common path owns preparation, ReviewPlan,
no-op, task registration, queue/cancellation, model preparation and publication
dispatch. ManagedHost retains real WriteGuard, original ReadSession and history;
the existing managed build wrapper and permission consumer remain authoritative.

The behavior deliberately becomes stricter: revision/history/lifecycle changes
are rejected, pending recovery is refused instead of reconciled, while compatible
metadata is captured immediately before publication. Review must approve that
behavior, not just file movement. Only the three existing modeling files and
three new modeling files listed above are in scope.

The host trait keeps backend authorization, original/latest snapshots, publication
guard and receipt typed. A future folder adapter can return its real CommitOutcome;
Published is not a confirmed success flag. Indeterminate UUID resolution must
retain the original opening and must never replay the command or apply an
unconfirmed candidate in UI. Folder durable state must use its session SQL/CAS,
while the runtime wrapper exposes only global task services.

Four proposed managed source/queue tests are included; they have not run. No
compilation, tests or production edits were performed. Only proposed copies
were formatted; read-only patch applicability passed. Existing real managed worker
tests and fresh-process folder model/edit/undo/redo/imported-STEP acceptance
remain required before claiming stage 4 or enabling production folder routing.
