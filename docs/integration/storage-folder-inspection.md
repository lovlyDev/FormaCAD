# Инспекция STEP исходной папки — точный патч

[English](storage-folder-inspection.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **применён после разрешения lovlyDev на все 10 файлов** 4 ноября 2026.
Строгая компиляция и native-регрессия прошли на `5899c64`. Основа патча —
`4e16dd42da581b664618837a2c0e1041acfc1e2c`, Issue #31 / PR #35.
Ответственный за реализацию — wenlixxq, согласование и приёмка — lovlyDev.

## Контракт

Существующий `inspect_model` сохраняет managed-вызов `{projectId, revisionId}`.
Folder-вызов требует дополнительно `{sourceId, expectedCommitId}` вместе.
Вызывающее окно предоставляет Tauri; `WindowSources::capture_read` проверяет
окно, исходное открытие, project ID и commit и возвращает связанный
`RepositoryRead`. На неподдерживаемой платформе или в сборке без native client
folder-вызов получает переведённый код `PROJECT_FOLDER_UNSUPPORTED`, не managed
fallback и не фиктивный `null`. Неполная пара полей отклоняется.

Оба backend используют один `run_bound`: сохранённая ревизия/описатель выбираются
из исходного снимка, байты читаются общим reader, original view проверяется после
очереди CAD и после настоящего worker. Алгоритм вычисления свойств и worker API
сохраняются. Остаётся финальная проверка байтов/отмены. Legacy-код ошибки
`PROJECT_SNAPSHOT_CHANGED` при inline checksum mismatch сохраняется через явный
адаптер existing integrity error. Writer grant или recovery не появляются.

Frontend `inspectSavedModel` принимает необязательный существующий
`FolderSourceRef`; старые вызовы совместимы. Browser с folder ref отказывает через
уже существующий перевод. Кнопок/новых пользовательских строк патч не добавляет.

## Десять точных путей

Все Rust-пути относительно `apps/desktop/src-tauri/src/`:

- `project_repository/window_sources/mod.rs`: регистрация capture-модуля и вызов общего метода.
- `project_repository/window_sources/supported.rs`: перенос capture в платформенно стабильный модуль.
- `project_repository/window_sources/assets.rs`: использование того же `capture_read`.
- `project_repository/window_sources/assets/tests.rs`: существующие проверки через публичный host-вход.
- новый `project_repository/window_sources/capture.rs`: общий host capture/refusal для всех платформ.
- `model_inspection.rs`: необязательные source-поля существующего IPC.
- `model_inspection/read.rs`: один общий run для managed/FCP1, прежний test seam сохранён.
- `model_inspection/read/tests.rs`: только `pub(super)` для двух existing fixture helpers.
- новый `model_inspection/read/folder_tests.rs`: настоящий worker и позднее закрытие папки.
- `apps/desktop/src/lib/modelInspection.ts`: необязательная source-ссылка клиента.

## Проверки и незавершённые критерии

Точный diff: [storage-folder-inspection.patch](storage-folder-inspection.patch).
На `5899c64fe30bd5f83beb31caceaf7884acd56948` прошли оба строгих Clippy и
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187435).
[Native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187467) подтвердил
`independent_step_properties_use_original_folder_and_refuse_late_close`.

Новая native-регрессия строит настоящий STEP 40×20×10, сохраняет FCP1 и создаёт
конфликтующую managed-строку с тем же ID, но заведомо неверными STEP-байтами.
Folder inspection должна получить объём 8000 мм³ и правильные bounds. Затем
после настоящего worker закрывается исходная папка: поздний результат должен
быть отвергнут. SQL папки и app state сравниваются с исходными байтами/состоянием.
Существующие native-тесты managed-пути не ослабляются. Native CI указанного SHA:
535 library PASS / 18 ignored, 11 smoke PASS, 41 host consumer PASS и три
отдельно запускаемых folder roundtrip PASS. Это доказательство backend-маршрута;
пользовательский интерфейс папок остаётся отдельным незавершённым критерием.

В исходной ревизии этого патча группировка задач инспекции использовала project ID.
Последующий [task-scope патч](storage-folder-task-scope.md) переводит FCP1-инспекцию
на UUID открытия. Workspace task filtering ещё требуется до включения UI папок.
Этот патч не завершает workspace/source state, остальные
geometry consumers, model writes/permissions, history IPC, recent index или весь
этап 4. §5 ТЗ требует отдельного согласования model_inspection и frontend API.
Native/Cargo/C++/worker protocol/permissions/SQL-схемы/версии неизменны.
