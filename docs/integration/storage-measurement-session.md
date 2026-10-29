# Согласование сеанса точных измерений / Exact measurement session integration

## Русский

Статус: **согласовано lovlyDev 29 октября 2026 и применено к шести файлам в задачной ветке**.
Результаты compile/worker тестов ещё требуются. Приёмка и
владелец общей интеграции — lovlyDev; область native/worker закреплена в Issue #3.
Исполнитель предложения — wenlixxq. [Точный патч](storage-measurement-session.patch)
подготовлен относительно `main` @ `4834e1020f755d4a158a105fc63965315d9c78c1`.
Для гарантии lifecycle он зависит от исправления общего `CommittedSnapshot`
в коммите `90d37dd10e0ab1c18c6092363006ce9213ca6374` ветки `codex/storage-week`:
этот снимок должен удерживать исходный `ReadSession` и проверять его при recheck.
Патч не заменяет указанную зависимость и сам по себе на прежнем адаптере
не доказывает отклонение close/reopen.

Проблема: `model_measurement::snapshot::read` оставляет только `Project`, удаляя
исходный сеанс. Финальная проверка заново захватывает проект; если модель и её
контрольные суммы не менялись, ответ старого worker переживает close/reopen.

Предложение сохраняет весь `CommittedSnapshot` в `CapturedMeasurement`, не
клонируя ещё один `Project` или STEP-буфер. Исходный token проверяется после чтения
исходных файлов и до/после повторного захвата измерительных входов. Имеющийся
финальный `capture::recheck` после worker поэтому проверяет именно исходный
сеанс. Тот же capture используется парными измерениями и сечением по грани:
их изменения ограничены доступом к проекту внутри сохранённого снимка.

Сохраняются проверки expected revision, authored body, document/source hash и
размера STEP, preview seal, imported assets, request/report echo и cancellation.
Worker protocol и C++/OCCT не изменяются. Ошибки проходят прежний перевод
`MEASUREMENT_STALE` / `MEASUREMENT_RECOVERY_REQUIRED`; новых пользовательских
строк, IPC, настроек или изменений SQL нет. Отказ ничего не прикрепляет к модели.
Общий снимок строже прежнего сравнения геометрии: изменение метаданных проекта
тоже может потребовать повторного запроса. Гарантия optimistic, после последней
проверки блокировка внешнего писателя не удерживается.

Патч затрагивает ровно шесть файлов: `model_measurement/{snapshot,capture,build,build_tests}.rs`,
`model_pair_measurement/build.rs`, `model_face_section/build.rs`. Он включает
регрессию с настоящим worker: hook вызывается после успешной проверки отчёта,
закрывает/открывает исходник без изменения сохранённых байтов, старый ответ
отклоняется, SQL/история/артефакты остаются прежними, новый запрос проходит.
Снимок фикстуры намеренно сравнивает данные проекта, а не меняющуюся подсказку
owner JSON. Дочерние процессы создаёт штатная CAD-фикстура; чужие процессы
не завершаются.

Проверено только: `git apply --check docs/integration/storage-measurement-session.patch`
на текущих неизменённых shared-файлах и rustfmt временных копий. Патч **не
скомпилирован и не исполнен**, PASS для S28 не заявлен. После согласования
владелец интеграции применяет его к точной совместимой основе и выполняет:

```sh
git apply --check docs/integration/storage-measurement-session.patch
git apply docs/integration/storage-measurement-session.patch
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --features native-worker-client -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-worker-client --lib model_measurement::build::tests
```

Последней команде нужен реально собранный совместимый worker и его DLL в
штатном расположении либо `FORMA_MEASUREMENT_TEST_WORKER`. Пропуск или отсутствие
worker не является проверкой. Дополнительно нужны существующие geometry,
invalid-report, corruption, cancellation, pair-measurement и face-section suites;
добавить close/reopen после worker для парного измерения и сечения по грани,
закрытие в очереди и смену источника при одинаковых IDs. Провести UI-проверку
прежнего сообщения stale и отсутствие принятого результата на RU/EN. Зафиксировать
SHA, feature set, реальные passed/failed/skipped и ограничения оборудования.

## English

Status: **proposed integration, not applied**. lovlyDev owns acceptance and the
shared integration; Issue #3 owns native/worker code. wenlixxq prepared the
[exact patch](storage-measurement-session.patch) against `main` commit
`4834e1020f755d4a158a105fc63965315d9c78c1`. Lifecycle fencing also requires the
shared `CommittedSnapshot` fix from `90d37dd10e0ab1c18c6092363006ce9213ca6374`
on `codex/storage-week`, which retains and rechecks the original `ReadSession`.
Applying this patch alone to the old adapter does not establish close/reopen safety.

The current measurement reader drops its snapshot and retains only `Project`.
Recapturing identical bytes later therefore accepts a result from an earlier
opening. The patch keeps the original `CommittedSnapshot` inside
`CapturedMeasurement`, without an additional full project/STEP clone, checks it
after file reads and before/after recapture, and uses the existing post-worker
acceptance check. Pair measurements and face sections share this capture; their
edits only access the project nested inside the retained snapshot.

Expected revision, authored body, document/source checksum and STEP size,
preview/import seals, request/report echo and cancellation checks remain. No
worker protocol, C++/OCCT, IPC, SQL or UI settings change. Existing localized
`MEASUREMENT_STALE` and `MEASUREMENT_RECOVERY_REQUIRED` errors remain. Rejection
does not attach results to the model. Full snapshot validation is stricter than
geometry-only checks: metadata edits may require retry. Rechecks are optimistic
observations, not a lease against later external changes.

The six-file patch includes a real-worker regression: reopen after a verified
report, reject the old result, compare unchanged project/SQL/history/artifacts,
then accept a fresh request. Its fixture excludes changing owner hints from the
saved-data comparison. It uses the existing isolated worker fixture and never
terminates unrelated processes.

lovlyDev authorized these six files on 29 October 2026; the patch is now applied
in the task branch. Compilation and real-worker results are still required.
Initially only patch applicability and rustfmt of temporary copies were checked. **No
compilation or execution is claimed**, and S28 remains unverified. After owner
agreement, use the commands above against the exact integration SHA. The test
requires a compatible real worker and DLLs in the normal location or via
`FORMA_MEASUREMENT_TEST_WORKER`; missing/skipped worker coverage is not PASS.
Run existing geometry, invalid-report, corruption, cancellation, pair and face
section suites as well. Add late close/reopen tests for pair measurements and
face sections, closure while queued, and source switches with identical IDs.
Check stale UI behavior in RU/EN and record SHA, features, passed/failed/skipped
counts and actual hardware limits.
