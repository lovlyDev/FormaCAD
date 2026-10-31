# Pure model preparation / Чистая подготовка сохранённой модели

Base: `1ebfccde1b36f819657fbe4788f6873213b87723`.
Patch: [storage-model-preparation.patch](storage-model-preparation.patch).
Status: prepared from exact base copies, NOT applied. No formatter, build, or tests were run by this preparer. `git apply --check` passes. Four source/test paths, 202 insertions and 40 deletions before formatting.

## RU

Это узкий slice A для этапа 4, подготовленный заранее для согласования общих modeling-файлов. Он **не реализует** внешний folder backend и не переводит этап 3/4 в состояние готовности.

Существующий `modeling/commit.rs` сейчас одновременно собирает следующую сохранённую модель и вызывает managed `projects::persist`. Патч выносит только сборку Project + normalized pending blobs в новый `modeling/prepared_change.rs`. Managed commit вызывает чистую подготовку, потребляет её результат и продолжает тот же `projects::persist(state, project, pending, None)`. Старый `program_for_revision` и его тест остаются на прежнем месте без изменения поведения.

`PreparedModelChange` имеет закрытые поля, не реализует Serialize/Deserialize и конструируется только через `pub(super) prepare` внутри modeling. Единственный crate-visible consuming accessor `into_parts` отдаёт Project и pending bytes. Кандидат не является commit receipt, writer lease, permission grant или PreparedHistory. Следующий writer обязан заново проверить source identity/lifecycle/location/expected commit, права, оригинальный captured baseline, историю и все данные; возможность получить части кандидата не разрешает обойти эти проверки.

Чистый preparer получает явный timestamp. Host берёт время один раз непосредственно перед вызовом, и одно значение используется для revision.created_at и project.updated_at. В исходном коде timestamp снимался после создания двух data URL; его выбор переносится на несколько строк раньше в host wrapper. При одинаковых timestamp/входах структура и bytes подготовки сохраняются; идентичность времени разных реальных запусков не обещается.

Сохраняются имена model-{job}.step/preview-{job}.glb, revision UUID/job, parent текущей ревизии, сохранённый program/prompt, program_base, defaults параметров, прежняя переписка/thumbnail/exports/старые ревизии и file references. Порядок — сборка Project → normalize → cancellation — прежний. После него добавляется чистый `Project::validate` перед возвратом кандидата: managed persistence уже валидирует Project, теперь недопустимый кандидат не может считаться подготовленным даже до persistence. Ошибка normalization по-прежнему опережает cancellation. Ни файловых, ни SQL, ни clock-вызовов в preparer нет.

Нормализатор не меняется. Он может вернуть два одинаковых pending hash/byte элемента для одинаковых STEP/GLB bytes; патч сохраняет этот контракт и не заявляет новую дедупликацию. Оба descriptor используют один content address; реальную CAS-дедупликацию выполняет storage publication, не данный чистый модуль.

Шесть подготовленных unit-тестов проверяют:

- неизменность исходной Project-копии, metadata и старых revision/file references; новое parent/program/job/timestamp; прежние defaults и точные pending bytes/hash/size;
- одинаковый content address outputs без изменения duplicate-pending контракта;
- cancellation без успешного кандидата и изменения исходника;
- отклонение duplicate revision ID;
- отклонение недопустимой project metadata;
- сохранение порядка normalization error перед cancellation.

Байты STEP/GLB синтетические. Тесты не вызывают ядро и не доказывают геометрическую корректность, настоящий CAD commit, сохранённую SQL-историю или production CAS. Присутствие тестов не равно PASS. После согласования main должен применить патч, выполнить rustfmt/Clippy и действующие managed/native host тесты; отдельно нужен новый CI проверяемого head.

## Точные общие пути

1. `apps/desktop/src-tauri/src/modeling.rs` — crate-visible регистрация prepared_change module, без IPC-изменений.
2. `apps/desktop/src-tauri/src/modeling/commit.rs` — прежний managed persistence wrapper; старый program_for_revision сохранён.
3. `apps/desktop/src-tauri/src/modeling/prepared_change.rs` — новый чистый preparer и privately constructed candidate.
4. `apps/desktop/src-tauri/src/modeling/prepared_change/tests.rs` — новые чистые регрессии.

Ни `native/*`, ни worker/protocol/C++/Cargo, ни build.rs/execution, permissions, projects persistence, схемы, версии, UI или migrations не меняются. Native imported-STEP seam и source-aware permission/task contract остаются отдельными согласованиями. Кандидат не маскируется под самостоятельный folder commit.

## EN

This slice extracts saved-model assembly from the existing managed commit into a pure module. The managed wrapper consumes the private candidate and calls the same projects::persist route. Filenames, parameter defaults, metadata, revision parent/job/program binding, normalization and cancellation ordering are retained. program_for_revision stays unchanged.

The host supplies one explicit timestamp, sampled immediately before pure preparation rather than after constructing data URLs; deterministic equal-input behavior is preserved, not wall-clock equality across separate executions. A Project::validate check is added after the existing normalization/cancellation sequence, before candidate success; managed persistence already validates projects.

PreparedModelChange has private fields and construction, no serialized authority, and one consuming crate-visible into_parts accessor. It is not a commit receipt, lease, permission or history intent. The actual backend must still validate original source/session/location/commit expectations and history under its own genuine writer authority. Normalization keeps its existing duplicate pending entries for identical bytes; equal content addresses are not a claim that this pure module publishes or deduplicates CAS.

Six synthetic pure tests cover source/metadata/revision-reference preservation, exact output descriptor and blob hashes/sizes, shared content addresses, cancellation, invalid candidates and error ordering. They were prepared but not executed. No geometry/kernel, SQL history, filesystem publication or external-folder success is claimed. Only git apply --check passed; main owns formatting, compilation and real host/worker validation after the four paths receive approval.