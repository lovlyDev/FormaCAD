# M2 history amendment and storage publication / M2 и публикация хранилища

Status: applied after lovlyDev explicitly approved all four targets on 3 November
2026; integrated with accepted main. Runtime verification remains required.
[Exact patch](storage-main-amend.patch).
Task base: `e965129`, accepted owner integration: `a4aec3e6`.

## RU

В принятом main новый M2-маршрут передаёт `AmendCapture` в старый CAD commit.
Наша ветка уже использует общий source-aware orchestration. Простое сохранение
нашего `apply.rs`/`commit.rs` при merge потеряет поведение M2; сохранение старого
main pipeline потеряет проверки исходного сеанса. Этот патч сохраняет оба.

Четыре общие цели, отдельно согласуемые по §5 ТЗ:

- `apps/desktop/src-tauri/src/modeling/apply.rs` — поле запроса из main и передача его копии managed host до перемещения запроса;
- `apps/desktop/src-tauri/src/modeling/commit.rs` — подготовка amend intent из готового кандидата и существующий `persist_with_history`;
- `apps/desktop/src-tauri/src/modeling/repository_apply/managed.rs` — чтение истории под publication mutex, повторная проверка исходного lifecycle и полного latest snapshot после SQL await;
- `apps/desktop/src-tauri/src/modeling/repository_apply/tests.rs` — явный `None` у существующих обычных запросов.

Патч проверен `git apply --check` на задачной основе, его Rust-синтаксис
отформатирован rustfmt. Это не доказательство компиляции: типы и `prepare_amend`
приходят из принятого main. При merge нужны версии из патча для первых двух
файлов. Затем обязательны strict Clippy, существующие M2 history/worker проверки,
storage lifecycle/queue регрессии и настоящий folder roundtrip на итоговом SHA.

Обычные запросы продолжают вызывать `persist`; amend использует существующую
транзакционную проверку `PreparedHistory`, не отдельную запись курсора. Общий
CAD алгоритм, разрешения, worker, native, Cargo, IPC и версии патч не меняет.
Новый UI/backend folder amend этим патчем не объявляется подключённым.
Конфликт App при merge должен сохранить owner topbar menu и storage opening
notice без второго экземпляра меню; i18n объединяется без удаления ключей.

## EN

Accepted main M2 carries `AmendCapture` through its older CAD commit pipeline.
The task branch already uses the common source-aware orchestrator. Keeping only
one side would lose either M2 amendment or the retained-source checks. The four
targets above preserve both: carry the capture into the managed host, load history
under its publication mutex, recheck the original lifecycle and latest snapshot
after that SQL await, prepare the existing amend intent and persist it atomically
with the candidate. Ordinary edits retain normal persistence.

The exact patch applies to task base `e965129`; its syntax is rustfmt checked.
It needs accepted main's history types and is not yet compilation/runtime proof.
After resolving the two modeling conflicts using the proposed content, strict
Clippy, M2 history/worker suites, source lifecycle/queue tests and actual folder
CAD execution must pass on the resulting SHA. Existing fixture hosts explicitly
use no amendment. Native, Cargo, permissions, IPC and versions are unchanged.
Folder amendment/UI availability is not claimed. The App resolution must keep
the owner topbar menu and the storage opening notice without duplicating menus;
translation catalogs retain both sets of keys.
