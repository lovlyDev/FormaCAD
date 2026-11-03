# Rust API самостоятельной папки FCP1

[Документация](index.md) · [English](../en/storage-folder-host-api.md) · [Формат](storage-formats.md)

`project_repository::sqlite_folder::api` подключает backend к обычной Rust-сборке
Windows и Linux x86_64/aarch64. Текущая ревизия ещё требует компиляции и runtime
проверок. Это доверенный host API: UI/IPC и app index пока не подключены. Выбранная
папка не копируется в AppData; LegacyManaged и V2 reader продолжают работать отдельно.

## Чтение

Host открывает явно выбранный путь через `FolderProject::open` и получает
непрозрачный `ReadView`. `capture_metadata` проверяет Project, cursor и объявленные
commit seals без чтения таблицы событий; `capture` дополнительно выгружает и
проверяет полный archive. View сохраняет физическое открытие и исходный saved
state, не сериализуется и не даёт права записи.
`read_asset`, `read_history`, `recheck`, `resolve_commit` используют исходный view.
Геттеры project/history/commit возвращают данные снимка; перед принятием
отложенного результата всё равно нужен `recheck`.

Общий `ProjectRepository::independent_folder` делегирует этому же backend без
app SQL или managed row. `close` отзывает только конкретный repository handle:
другое открытие того же ID остаётся действительным, старый snapshot не переходит
в новый handle. Проверки окружают awaited операции. Reader не получает writer
lease, не исправляет journal и не запускает recovery.

Общий `capture` использует metadata-режим; `capture_with_history` — полный.
`read_history` выполняет полную проверку под исходным открытием и сравнивает
commit/project/cursor/archive seals с исходным view, а не узаконивает новый
снимок. `approve_write` также требует полной проверки до выдачи grant.
Metadata view подтверждает объявленный hash архива, но не его байты: повреждение
events может не мешать метаданным, однако история, copy и запись получают отказ.
CAS read из metadata view проверяет собственные байты без выгрузки архива.
Project с метаданными ревизий и commit chain всё ещё читаются; это не O(1) и не
обещание одинаковой стоимости backend. Замеры новой ревизии остаются обязательны.

Ignored-тест `api::benchmark::measure_scoped_reads` измеряет реальные FCP1
`capture + recheck + drop` для metadata и complete: по 12 выборок с чередованием
порядка после прогрева, median/p95 отдельно для трёх размеров. Нагрузка повторяет
синтетические revision/chat/STEP-labelled fixtures прежнего сравнения; это не CAD.
Вспомогательная SQL-база подготовки удаляется до открытия FCP1, файл FCP1 до/после
чтений сравнивается по hash. Вывод `FCP1_READ_BENCHMARK` содержит объёмы Project,
archive и CAS, время первого API open и сырые распределения. Это debug/test
профиль с неочищенным кэшем файловой системы; RSS/time включают также подготовку.
Результат не доказывает скорость release-приложения, холодного диска или USB/NAS
и не уравнивает разные гарантии metadata и полного чтения. Workflow требует один
выполненный тест; текущие численные результаты ещё не получены.

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
Отдельный SQL authorizer запрещает чтение `project_history_events`: metadata
должен проходить, полный capture — отказывать. Fault-регрессии проверяют отказ
history/copy/write при повреждённом archive и сохранение исходного view после
внешнего commit; они не подменяют проверки флагом «архив не читался».
Для самого публичного `apply_model` выделен ignored-тест
`api::worker_tests::real_worker_host_api_roundtrip`: настоящая коробка 40×20×10,
no-op без grant, отказ изменению без grant, close и открытие отдельным процессом
с проверкой объёма 8000 мм³ и размеров через worker. Workflow явно требует один
выполненный PASS; тест ещё не подтверждён на текущей ревизии.
При `native-worker-client` API также предоставляет `SelectedStepFile`,
непрозрачный `StepRead`, одноразовый `StepDestinationApproval` и
`transfer_to_folder` с тем же `ModelRuntime`. Это прямой доступ к существующему
контроллеру [переноса STEP](storage-step-transfer.md), без старой app SQL и
восстановления параметрики. Hash/read и approve — блокирующие файловые операции;
UI host должен планировать их отдельно от своего потока интерфейса. Новая
синтетическая регрессия проверяет чужое открытие и файл, появившийся в назначении
после разрешения. Настоящий STEP roundtrip теперь вызывает этот публичный API.
Проверка native-client ревизии всё ещё обязательна.
Production UI/index, реальные USB/NAS и fixture 1.1 ещё
не приняты. Версии и релиз этот API не меняет.
