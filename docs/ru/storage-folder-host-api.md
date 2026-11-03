# Rust API самостоятельной папки FCP1

[Документация](index.md) · [English](../en/storage-folder-host-api.md) · [Формат](storage-formats.md)

`project_repository::sqlite_folder::api` подключает backend к обычной Rust-сборке
Windows и Linux x86_64/aarch64. Текущая ревизия ещё требует компиляции и runtime
проверок. Это доверенный host API: UI/IPC и app index пока не подключены. Выбранная
папка не копируется в AppData; LegacyManaged и V2 reader продолжают работать отдельно.

## Чтение

Host открывает явно выбранный путь через `FolderProject::open` и получает
непрозрачный `ReadView` вызовом `capture`. View сохраняет физическое открытие и
согласованный снимок SQL/history, не сериализуется и не даёт права записи.
`read_asset`, `read_history`, `recheck`, `resolve_commit` используют исходный view.
Геттеры project/history/commit возвращают данные снимка; перед принятием
отложенного результата всё равно нужен `recheck`.

Общий `ProjectRepository::independent_folder` делегирует этому же backend без
app SQL или managed row. `close` отзывает только конкретный repository handle:
другое открытие того же ID остаётся действительным, старый snapshot не переходит
в новый handle. Проверки окружают awaited операции. Reader не получает writer
lease, не исправляет journal и не запускает recovery.

Текущее ограничение производительности: FCP1 capture ещё проверяет полный
ограниченный archive истории и цепочку commit. Это не ленивое metadata-only чтение;
его оптимизация и замеры остаются пунктом приёмки. Managed capture сохраняет
своё чтение без event archive. Наличие общего API не доказывает равную стоимость
чтения разных backend.

## Запись и CAD

После явного решения пользователя host вызывает `approve_write` исходного view.
Одноразовый `WriteApproval` связан с оригинальным saved-state stamp. Он не
восстанавливается из project ID, UI token или настроек и не заменяет реальный OS
writer lease, получаемый отдельно при публикации.

`create` требует пустую папку и валидные project/assets/history; `commit`
публикует подготовленный backend candidate; `apply_history` использует общий
planner undo/redo. UI не должен передавать произвольный Project в эти доверенные
Rust entry points. `save_copy` не требует writer-права исходника, создаёт новую
identity и сохраняет переносимую историю.

`apply_model` принимает исходный view, `ModelIntent` и доверенный `ModelRuntime`
(task map, CAD gate, machine paths). Project ID и expected revision берутся из
исходного view. `FolderApplyHost` вызывает существующие `repository_apply::apply`
и настоящий `execution::build`, без второго алгоритма геометрии. No-op принимает
`None` вместо разрешения и не получает writer lease; изменение без разрешения
отклоняется. Runtime paths не принимаются из IPC.

## Исход публикации и проверки

`V2Backup::create` создаёт точную независимо перечитанную копию явно выбранного
опубликованного V2-сеанса. Это блокирующий файловый маршрут: будущий UI host
должен выполнять его на blocking worker. Объект proof не сериализуется;
`BackupRecord` — только сведения о расположении, manifest SHA, generation и ID,
которые не позволяют восстановить право на миграцию. `V2Backup::migrate`
использует тот же проверенный converter и отдельную пустую папку FCP1; исходник
и backup не становятся целью записи. Даже отказ назначения сохраняет backup
provenance в `MigrationReport`; новое открытие по тому же пути не наследует proof
старого opening. Два новых API-теста проверяют точность manifest, сохранение
project/history и отказ после close/reopen. Их текущий результат ещё не получен.

`Publication::Committed` содержит saved project и UUID, но не новую capability.
`RejectedBefore` сохраняет отказ до commit. `Indeterminate` сохраняет известный
UUID и техническую ошибку: запрещены автоматический повтор и заявление об откате.
`ModelChange::Published` сам по себе ещё не означает подтверждённую запись.
`resolve_commit` использует исходное открытие; повторное открытие требует нового
view и не возрождает старый grant.

Тесты API используют настоящую изолированную SQLite/CAS: reopen без app index,
отказ чужому opening/grant, read-only copy при чужом writer lease и чистый общий
reader. Fixtures синтетические и не заменяют настоящие worker roundtrips.
Для самого публичного `apply_model` выделен ignored-тест
`api::worker_tests::real_worker_host_api_roundtrip`: настоящая коробка 40×20×10,
no-op без grant, отказ изменению без grant, close и открытие отдельным процессом
с проверкой объёма 8000 мм³ и размеров через worker. Workflow явно требует один
выполненный PASS; тест ещё не подтверждён на текущей ревизии.
Production UI/index, STEP facade, реальные USB/NAS и fixture 1.1 ещё
не приняты. Версии и релиз этот API не меняет.
