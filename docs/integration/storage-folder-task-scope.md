# CAD-задачи конкретного открытия папки — точный патч

[English](storage-folder-task-scope.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **применён после разрешения lovlyDev на все 8 файлов** 4 ноября 2026.
Scope-assertions и native-регрессия прошли CI на `5899c64`. Основа патча —
`4fd64373fa0e029896f4a296456a567bae3d1775`, Issue #31 / PR #35.
Ответственный — wenlixxq; согласование и приёмка — lovlyDev.

## Контракт

FCP1 Opening уже создаёт непостоянный UUID `task_id`; FolderApplyHost уже
использует его как группу задач. Патч даёт тому же значению read-only accessor
через FolderSnapshot → ReadView → RepositoryRead. Новая идентичность, путь,
хеш или сохранённый token не создаются. Managed/V2 сохраняют группировку по
project ID; FCP1 получает группу именно исходного открытия.

Инспекция регистрируется в существующем CadTaskGate с этой группой. Глобальный
semaphore по-прежнему допускает одного тяжёлого worker; индивидуальный task UUID,
status/cancel API и worker protocol не меняются. Две папки с одинаковым project ID
имеют разные группы, одна папка сохраняет группу после commit, close/reopen
получает новую группу. Это совпадает с группой существующего FolderApplyHost.

`open_project_folder` дополнительно возвращает `taskScopeId`; FolderSourceRef
содержит это поле для будущей фильтрации task status в workspace. Сервер не
принимает его как authority или как аргумент выбора источника: чтение/инспекция
по-прежнему проверяют настоящее окно, sourceId и expectedCommitId. Значение не
сохраняется в Project, SQLite или app index и не даёт writer grant.

## Восемь точных путей

Rust-пути относительно `apps/desktop/src-tauri/src/`:

- `project_repository/sqlite_folder/session.rs`: accessor существующего task_id снимка.
- `project_repository/sqlite_folder/api.rs`: accessor ReadView.
- `project_repository/backend/read.rs`: общий task_scope_id с сохранением legacy-групп.
- `project_repository/window_sources/ipc.rs`: дополнительное поле ответа выбора папки.
- `apps/desktop/src/features/project-sources/contract.ts`: taskScopeId клиентской ссылки.
- `model_inspection/read.rs`: группа из bound reader вместо project ID.
- `model_inspection/read/folder_tests.rs`: проверка реальной регистрации группы во время worker.
- `project_repository/window_sources/assets/tests.rs`: разные группы одинаковых ID, стабильность при commit и новая группа при reopen.

## Проверки и ограничения

Точный diff: [storage-folder-task-scope.patch](storage-folder-task-scope.patch).
На `5899c64fe30bd5f83beb31caceaf7884acd56948` прошли строгая компиляция обоих
вариантов и [Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187435):
98 folder, 2 window/asset и 3 original-source refresh теста.
[Native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187467) подтвердил
`independent_step_properties_use_original_folder_and_refuse_late_close`, включая
проверку task group исходного открытия: 535 library PASS, 18 ignored; 11 smoke
PASS, 41 host consumer PASS и три отдельно запускаемых folder roundtrip PASS.

UI task filtering, остальные geometry consumers, workspace/model/history/index
ещё не подключены. Закрытие источника отзывает его результаты; API отмены
конкретной операции остаётся существующим task UUID. Схема статуса CadTaskGate,
native/Cargo/C++/protocol/permissions/SQL/версии не меняются. §5 ТЗ требует
согласования общего model_inspection, frontend contract и IPC-ответа.
