# Rust API самостоятельной папки FCP1

[Документация](index.md) · [English](../en/storage-folder-host-api.md) · [Формат](storage-formats.md)

`project_repository::sqlite_folder::api` подключает backend к обычной Rust-сборке
Windows и Linux x86_64/aarch64. Текущая ревизия ещё требует компиляции и runtime
проверок. Это доверенный host API: IPC выбора и чтения окна подключён ниже,
а продуктовый UI и app index ещё не завершены. Выбранная
папка не копируется в AppData; LegacyManaged и V2 reader продолжают работать отдельно.

## Чтение

### Обновление metadata через исходный источник

`RepositoryRead::recheck_source` проверяет исходное семейство reader, открытие
и физическое расположение, но не утверждает неизменность сохранённого проекта.
Он не заменяет `recheck` или проверку геометрических входов.
`recapture_metadata` получает отдельный актуальный metadata-view через тот же
удержанный repository, с проверкой исходного источника до и после. Путь или ключ
окна не ищутся заново, старый view не изменяется, закрытое открытие не оживает.
Обычные asset/history/IPC-чтения по-прежнему требуют полной saved-state проверки.

Этот маршрут нужен геометрическим потребителям, которые явно сравнивают исходную
ревизию, document/source/import/preview seals перед принятием metadata-сохранений.
Пока эти потребители к новому refresh не подключены. Новый metadata-view не
наследует LegacyManaged archive capture: история требует явного capture/read
согласно backend-контракту. Закрытие запроса отзывает всё семейство его производных
readers; независимые запросы того же окна продолжают работать.

Три регрессии на настоящих SQL/файлах проверяют LegacyManaged, V2 и FCP1:
metadata-публикацию, устаревание старого view, удержание source/epoch и отказ после
close/reopen. Также проверяются чистота чтения и разделение metadata/archive.
Точные Rust-проверки новой ревизии ещё ожидаются; наличие fixture не означает PASS.

### Точные свойства сохранённого STEP

Существующий `inspect_model` теперь принимает ту же необязательную пару ключа
источника и ожидаемого commit, что metadata- и attachment-IPC.
`inspectSavedModel` принимает `FolderSourceRef`; managed-вызовы сохраняют прежние
аргументы. Неверная/неполная ссылка папки отклоняется до инспекции; неподдерживаемая
сборка и browser-вызов с папкой используют существующую переведённую ошибку среды.

Оба backend используют один алгоритм инспекции через `RepositoryRead`. Он
выбирает сохранённую STEP-ревизию/описатель, проверяет байты общим reader,
перепроверяет исходный view после ожидания CAD gate и настоящего worker и
сохраняет byte/cancellation fences. Поля свойств и геометрический алгоритм
не изменены. Recovery и writer grant в этом маршруте отсутствуют.

Новая native-регрессия создаёт настоящий STEP 40×20×10 в FCP1, затем записывает
неверные STEP-байты в managed-строку с тем же project ID. Инспекция папки должна
получить 8000 мм³ и правильные bounds. Закрытие исходной папки после реального
worker должно отвергнуть поздние свойства без изменения SQL папки/app state.
Эта применённая ревизия пока не подтверждена native CI.

Для FCP1-инспекции группа задач теперь берётся из существующего UUID настоящего
открытия, как у FolderApplyHost. `open_project_folder` возвращает `taskScopeId`
вместе с sourceId/commitId/project; FolderSourceRef содержит это поле для
workspace task filtering. Оно не сохраняется и не принимается как authority
read/write/inspection IPC. Сервер выбирает группу из связанного view.
Managed/V2 сохраняют группы по project ID. Разные открытия папок с одинаковым
project ID имеют разные группы; commit сохраняет группу, reopen меняет её.
Глобальный semaphore одного worker и индивидуальный task UUID/cancel API прежние.

Разделение/стабильность групп и настоящая регистрация инспекции во время worker
прошли на `d242babdb83b75c49cdcbfcfd4e18b80776c07d3`:
[comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41813913525) и
[native](https://github.com/lovlyDev/FormaCAD/actions/runs/41813913691).
Эта точка предшествует добавлению metadata-refresh выше и не проверяет последующий код.
Workspace task filtering, остальные geometry consumers и UI папки не завершены.
Инспекция не завершает весь этап 4.

### Вложения исходного сеанса окна

`backend::RepositoryRead` удерживает `ProjectRepository` вместе с его исходным
`RepositorySnapshot`. Native reader может передать этот связанный объект через
очередь работы и после неё проверить то же сохранённое состояние. Managed-вызов
явно выбирает metadata capture или `capture_with_history`: metadata-only view не
получает архив неявно. Для FCP1 `from_folder_view` принимает именно исходный host
view и проверяет его принадлежность открытию до возврата. Путь не открывается
повторно, свежий capture не подменяет исходный.

Оконные metadata- и attachment-readers используют этот же общий маршрут. `close`
отдельного запроса отзывает только его reader; shared-источником продолжает владеть
окно. Закрытие источника отзывает все удержанные readers. Регрессия вложений также
проверяет отказ чужой паре opening/view, изоляцию закрытия запроса и отказ чтению
истории после внешней публикации вместо принятия её нового состояния. Эти изменения
связанного reader ещё требуют компиляции и тестов точной ревизии. Подключение
геометрических IPC-потребителей остаётся следующим общим интеграционным шагом.

Rust host может вызвать `WindowSources::read_asset(windowLabel, sourceId,
projectId, expectedCommitId, name)`. Метод использует общий с metadata-чтением
capture ожидаемого источника, выбирает описатель файла из сохранённого проекта
и читает через тот же `FolderProject`. Вызывающий не передаёт путь, размер или
checksum как authority. Существующий ограниченный CAS reader проверяет описатель
и байты; исходный view перепроверяется также после кодирования data URL. Managed
blob fallback и зависимость от app index отсутствуют. Неподдерживаемая платформа
отказывает этому маршруту.

Общий `read_project_file` IPC теперь принимает `sourceId` и `expectedCommitId`
вместе и использует настоящее вызывающее окно. Без обоих полей сохраняется
managed-маршрут; неполная пара или неверная ссылка папки получают отказ без
fallback. `readProjectFile` и `readProjectSnapshot` принимают общий необязательный
тип `FolderSourceRef`. Browser-вызов со ссылкой папки получает отказ до managed
lookup или invoke. Ошибки лимита источников и неподдерживаемой среды переведены
на RU/EN.
Регрессия на двух настоящих папках с одинаковыми project ID и именами файлов,
но разными байтами проверяет чистоту SQL, отказ чужому commit, отсутствующему
имени и обходу пути, затем отказ старому commit после настоящей публикации и
закрытому источнику. Это проверки байтов вложений; геометрия проверяется
настоящим worker ниже. На `1c15f96ba607d2745c26889c889696b27a15ee42`
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41807057154)
прошёл: 98 folder-тестов, два window/asset-теста, оба строгих Clippy-режима и оба
измерительных шага. Это не проверка применённого позже attachment IPC-патча или
desktop-сценария геометрии.

### IPC окна

Согласованный host state `WindowSources` регистрируется отдельно от `AppState`.
`open_project_folder` открывает системный выбор папки для вызывающего Tauri-окна
и возвращает `{sourceId, commitId, project}` либо `null` при отмене. Scope
закрепляется до ожидания диалога: поздний ответ после уничтожения окна не
создаст новое открытие. Путь не принимается в аргументах IPC.

Существующий `read_project_snapshot` принимает `{id}` для managed-проекта либо
`{id, sourceId, expectedCommitId}` для папки. Оба дополнительных поля обязательны
вместе. Чужое окно/ключ, неверный project ID или commit получают отказ; исходный
view перепроверяется после копирования ответа. Ошибка ключа не переключает
чтение на managed-строку. `close_project_source({sourceId})` отзывает выбранное
открытие; уничтожение окна и Drop host state отзывают все связанные источники.
Повторный label окна создаёт новый scope.

Это подключение ещё не добавляет видимую кнопку папки, IPC геометрии/записи модели
или recent index. Перед добавлением такого UI нужно перевести
`PROJECT_SOURCE_LIMIT` и `PROJECT_FOLDER_UNSUPPORTED` на границе показа RU/EN и
передавать identity источника через каждую связанную операцию. Технические детали
сохраняются отдельно. Неподдерживаемая платформа отказывает открытию папки, а не
подменяет его managed-хранилищем. Результаты новой регрессии окна и строгой
компиляции применённой ревизии ещё ожидаются; проверенная точка ниже предшествует IPC.

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
выполненный тест; численные результаты ниже относятся только к указанному SHA.

На `d6af326` замер выполнен в
[run 41739896245](https://github.com/lovlyDev/FormaCAD/actions/runs/41739896245),
job `126278491804`: 1 passed, 0 failed. Linux x86_64, debug/test, тёплый кэш;
по 12 выборок для каждого режима. Время включает capture, recheck и освобождение
view, но не подготовку набора и первый API open.

| Ревизий | Project / archive, байт | Metadata median / p95, мс | Complete median / p95, мс |
| --- | --- | --- | --- |
| 4 | 3893 / 1103 | 8.550 / 8.773 | 9.987 / 10.265 |
| 32 | 33535 / 8161 | 18.428 / 19.753 | 23.283 / 25.000 |
| 128 | 133911 / 32355 | 50.621 / 54.103 | 65.600 / 75.710 |

Независимость от удалённой setup SQL и неизменность FCP1 подтверждены внутри
замера. В том же run прошли strict Clippy с `native-worker-client` и без него,
92 обычных теста
папочного backend (6 ignored) и прежнее сравнение storage-представлений.
Это измерения разных объёмов проверки, а не разрешение ослаблять complete
маршрут записи или доказательство native CAD/установленного приложения.

## Запись и CAD

### Реестр открытий окна

`FolderSources` — непостоянный Rust host-реестр, по одному экземпляру на окно.
`open` принимает только уже явно выбранную host папку и возвращает `OpenedFolder`
с исходным `ReadView` и непрозрачным `FolderSourceId`. `lookup` возвращает Arc
того же открытия, не открывает путь заново и не использует project ID как ключ.
Две папки с одинаковым project ID остаются разными источниками. `close` немедленно
отзывает исходный сеанс, а `close_all` навсегда закрывает область этого окна,
включая pending открытия; новое окно создаёт новый реестр.

Общий читатель `ProjectRepository::from_folder_source` принимает настоящий Arc
из этого реестра. Он не открывает путь повторно и не создаёт managed-проект.
Каждая обёртка запроса связывает собственные снимки; закрытие обёртки отзывает
только её снимки. Временем жизни источника продолжает владеть реестр окна:
закрытие ключа отзывает все обёртки, повторное открытие не оживляет старые снимки.
Отдельный конструктор `independent_folder` по-прежнему владеет своим открытием
и закрывает его вместе с handle. Регрессия на настоящей папке проверяет изоляцию
обёрток и общий отзыв источника через единый маршрут чтения истории.

В одном реестре не более 16 готовых и pending источников вместе. Слот резервируется
до файлового IO, mutex карты не удерживается во время SQL/диска/worker/await.
Отмена освобождает свой слот; после публикации в карте, но до возврата результата,
guard также удаляет и отзывает только собственное открытие. Физическое закрытие
SQL-соединения происходит после выхода из map lock и может завершиться позже.
После успешного ответа ответственность за закрытие принадлежит host: он должен
закрывать поздний неиспользованный результат и все источники при закрытии окна.

Ключ включает область окна, монотонный номер без wraparound и случайный nonce
каждого открытия. Знание соседнего номера не даёт действительного ключа. Старые
ключи после close/reopen не переиспользуются; чужая область и поддельный nonce
отклоняются. Эти ключи не сохраняются в index/settings и не являются writer grant.
`WriteApproval` по-прежнему требует отдельного host решения и полного source
recheck. Внутренний код `PROJECT_SOURCE_LIMIT` пока доступен только Rust host API;
при согласованном UI/IPC подключении ему нужен RU/EN перевод на display boundary.
Регрессии используют реальные папки и production RAII-reservations: одинаковые
project ID, чужой nonce/окно, close/reopen/drop, quota до IO и отмена после publish.

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
project/history и отказ после close/reopen; доказательства для точной ревизии ниже.

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
выполненный PASS; проверенная ревизия указана ниже.
При `native-worker-client` API также предоставляет `SelectedStepFile`,
непрозрачный `StepRead`, одноразовый `StepDestinationApproval` и
`transfer_to_folder` с тем же `ModelRuntime`. Это прямой доступ к существующему
контроллеру [переноса STEP](storage-step-transfer.md), без старой app SQL и
восстановления параметрики. Hash/read и approve — блокирующие файловые операции;
UI host должен планировать их отдельно от своего потока интерфейса. Новая
синтетическая регрессия проверяет чужое открытие и файл, появившийся в назначении
после разрешения. Настоящий STEP roundtrip теперь вызывает этот публичный API.
Результат проверки native-client для точной ревизии указан ниже.
Production UI/index, реальные USB/NAS и fixture 1.1 ещё
не приняты. Версии и релиз этот API не меняет.

### Проверенная контрольная точка backend

На `c147bf9ca2b9e6811ed945feaa203845c6827227` успешно завершились три workflow:
[Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/41744522132),
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41744514043)
и [Storage native consumers](https://github.com/lovlyDev/FormaCAD/actions/runs/41744513890).
В скачанных логах native job `126302133894` и comparison job `126294056666`
зафиксированы выполненные проверки, а не только регистрация тестов.

| Проверка | Наблюдаемый результат |
| --- | --- |
| Folder packet без native client | 98 passed, 0 failed, 6 ignored |
| Строгий Clippy | Успех без native client и с native-worker-client |
| Измерения FCP1 scoped reads и представлений хранилища | По одному выполненному PASS |
| Native library packet | 529 passed, 0 failed, 18 ignored |
| Worker smoke packet | 11 passed, 0 failed |
| Семь host integration suites | 41 passed, 0 failed |
| Independent folder CAD roundtrip | Один явно выполненный PASS |
| Public folder host API roundtrip | Один явно выполненный PASS |
| Selected STEP transfer roundtrip | Один явно выполненный PASS |

Independent roundtrip строит настоящие модели шириной 40, затем 80, проверяет
геометрию, выполняет undo до 40, закрывает источник, проверяет сохранённую модель
в новом процессе с nonce, открывает её вновь и выполняет redo до 80. Публичный
API и перенос выбранного STEP также используют настоящий собранный worker и
проверку сохранённой геометрии в отдельном процессе. Явные ignored-тесты запущены
отдельно; числа ignored обычного пакета не означают выполнение всех пропусков.

Это доказательство backend-маршрутов в Linux CI на указанном SHA. Оно не
подтверждает Windows desktop picker, подключённый UI/IPC, физические USB/NAS или
настоящий fixture миграции 1.1. [Патч окна](../integration/storage-folder-window.md)
на этой контрольной точке оставался неприменённым артефактом для согласования.
Этап 4 остаётся открытым; эти backend-результаты не завершают всю программу.

## Измерения исходной папки через существующие команды

`measure_model_reference`, `measure_model_pair`, `section_model_reference` и
`snap_model_reference` принимают необязательные `sourceId` и `expectedCommitId`.
Ссылку получают только из результата выбора папки в текущем окне. Оба поля
отсутствуют — прежний managed-маршрут; неполная пара отклоняется. Host проверяет
исходное окно, проект и commit и передаёт первоначальный RepositoryRead в CAD.
Ошибка источника не вызывает чтение managed-проекта с тем же ID. taskScopeId
служит группировкой задач и не передаётся как право выбора источника.

При ожидании CAD изменения имени/сообщений допускаются лишь после recapture через
тот же исходный reader и сравнения первоначальных геометрических bindings.
Close/reopen, подмена папки или изменение геометрии отклоняют поздний результат.
STEP читается через backend с проверкой membership/size/hash и передаётся
существующему native stager. Измерения не сохраняют модель, историю или SourceRef.

Низкоуровневые frontend API принимают необязательный FolderSourceRef. Контролы
workspace ещё должны передавать его и инвалидировать selection/request state при
смене источника; видимой завершённой функции папок это пока не создаёт.
Без native client запрос папки получает переведённый PROJECT_FOLDER_UNSUPPORTED.
Основа reader/staging прошла [native CI на 64bde10](https://github.com/lovlyDev/FormaCAD/actions/runs/41908269630):
оба новых STEP/FCP1 теста выполнены. IPC на `5cb38ef` также прошёл
[native-проверки](https://github.com/lovlyDev/FormaCAD/actions/runs/42042470156);
GUI-маршрут остаётся отдельным условием. Детали: [контракт IPC](../integration/storage-measurement-source-ipc.md).

## Preview исходной папки

`preview_model` и `cancel_model_preview` принимают ту же необязательную пару
`sourceId`/`expectedCommitId`. Host проверяет реальное окно, проект и commit.
Запросы без пары сохраняют managed-маршрут; неполная или устаревшая пара получает
отказ. Источник, program hash и проверенный GLB остаются связанными одним запросом.
Preview не публикует ревизии, не расходует grants и не записывает модель/историю.

Один общий host держит исходный RepositoryRead, проверяет импортированные STEP
через backend, использует существующий native stager/worker и перепроверяет
снимок после worker и после последнего await очистки task map. Изменение metadata,
геометрии, close/reopen или подмена источника инвалидируют результат. Для FCP1
task/cancel group — UUID открытия; папки с одинаковым project ID разделены.
Общий CAD semaphore и индивидуальные cancellation tokens сохраняются.

Frontend preview/cancel API принимают необязательный FolderSourceRef. Для отмены
нужна действительная ссылка на текущий commit; устаревшая ссылка не переключается
на другое открытие. Без native среды запрос папки получает существующую переведённую
ошибку PROJECT_FOLDER_UNSUPPORTED. Входы worker находятся только в app-level
`.transient/previews`; при завершении Workspace удаляет их.

На `4b96158` [native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/42050145085)
выполнил все три preview-регрессии: настоящий импортированный STEP, позднее закрытие
во время worker/cleanup await и две папки с одинаковыми IDs и раздельной отменой.
Отдельный запуск дал ровно три PASS; общий library-пакет — 540 PASS / 18 ignored.
Подробности — [контракт preview](../integration/storage-preview-source.md).
Workspace теперь хранит source binding открытия и использует его для invalidation
сцены и основных callbacks. Видимые folder controls и полный маршрут записи ещё
не подключены; эта интеграция не завершает этап 4.

## Привязка renderer к открытию источника

Workspace публикует Project вместе с process-local `ProjectSourceBinding`.
Явное managed/folder открытие создаёт новое семейство запросов; metadata refresh
сохраняет его. UUID renderer служит только для отказа позднему callback и не
заменяет проверку host source/window/commit или writer permission. Binding и
source tokens не сохраняются в preferences или проекте.

При смене источника сбрасывается topology/body selection, прежняя геометрия сразу
скрывается, поздние объекты освобождаются. CommittedScene учитывает opening ID
дополнительно к revision/asset seals. Папочный modelLoader читает host-вложение
даже при inline-кандидате и проверяет тот же source/commit после декодирования.
Готовая геометрия сохраняется при metadata-only обновлении; pending IO получает
новую commit-ссылку. Загрузка не подставляет managed проект с тем же ID.

Managed save, navigation, revision events и основные CAD callbacks проверяют
исходное открытие; старый ответ не заменяет folder-view. Folder read reference
не вызывает managed persistence и не получает managed writer access. Статус доступа
также учитывает открытие, чтобы старый write-mode не отображался во время повторного
acquire того же ID. Ошибка PROJECT_SOURCE_WRITE_NOT_AUTHORIZED переведена RU/EN.

Project-scoped folder-черновики корневого окна остаются в RAM; managed preferences
для них не читаются и не пишутся. Их семейство включает открытие и ключ preference,
поэтому переход к другому ключу ревизии отклоняет старый setter. Это основа renderer,
а не уже доступный folder writer UI. Подключение editor/AI preview и остальных
measurement/history/import/export consumers идёт отдельными согласованными патчами.
Точный состав и проверки: [контракт состояния workspace](../integration/storage-workspace-source-state.md).

## Оконная операция с исходным снимком

`WindowSources.capture_operation(window, sourceId, projectId, expectedCommit)`
возвращает Rust-only `FolderOperation`: исходный `Arc<FolderProject>` и его
`ReadView` остаются вместе. Окно проверяется через собственный registry; другой
источник с теми же project IDs не подставляется. После значимого ожидания host
проверяет `recheck` исходного объекта; закрытие окна или открытия отзывает его.
Метод `into_read` переводит тот же захват в общий RepositoryRead. Существующие
оконные readers используют этот маршрут без дополнительного чтения архива истории.

Захват не является writer lease или пользовательским разрешением. Только trusted
host **после собственного явного решения о разрешении операции** вызывает
`approve_write`; renderer-ключ или переданный boolean не заменяют это решение.
Объект не сериализуется, не сохраняется и не экспортирует отдельно заменяемые
source/view. Методы `commit`, `apply_model` и `apply_history` передают первоначальный
снимок и непрозрачный WriteApproval существующему транзакционному backend. CAD
использует прежний общий executor; `None` допускает только прежний no-op маршрут.
Raw Project от renderer не является допустимым подготовленным кандидатом host.
Writer OS-lock по-прежнему приобретается существующим publisher перед записью.

`save_copy` использует тот же исходный снимок, не запрашивает запись исходника
и создаёт независимую identity в отдельно выбранной host папке. Здесь нет нового
IPC или видимой кнопки: permission UI и применение оконного объекта из writer IPC
ещё требуют подключения и согласования общих файлов.

Четыре теста `window_sources::operation::tests` проверяют реальные SQL/CAS:
непереносимость approval между папками с одним ID; старый объект после закрытия и
переиспользования window label; отказ запоздалому approval после нового commit;
read-only копию при занятом OS writer-lock исходника с проверкой неизменности его
SQLite. Они не выдают metadata-изменение за CAD-геометрию. Выполнение нового набора
проверяется в CI текущего коммита; наличие тестов и rustfmt само по себе не PASS.

## Preview редактора и AI-review

После согласования восьми frontend-файлов `useModelPreview` получает binding
исходного открытия. ProgramDialog передаёт его в preview/cancel и держит folder
черновик в RAM через source preferences. При смене открытия старый preview сразу
скрывается и освобождается; отмена относится только к исходному source/task group.
Поздние ответы и ошибки не отображаются в новом открытии с одинаковыми ID. Явно
отсутствующий или чужой binding не превращается в managed IPC. Metadata-commit
перезапускает строгий preview редактора; изменение текста того же источника сохраняет
прежнюю последовательную очередь и debounce.

AI-review сохраняет исходный binding до confirmation-await. Его кандидат и отдельная
сохранённая GLB читаются из этого источника; исходная сцена workspace не переносится
в окно review. Смена открытия либо commit блокирует принятие, включая metadata-only
commit: старый review не получает новую authority через recapture. Требуется новое
review. Baseline cache также разделён по открытию и commit и освобождает поздние
объекты. Согласованный патч и отдельное исправление commit-регрессии описаны в
[контракте frontend preview](../integration/storage-preview-renderer-source.md).

Перед применением все три CI основы `6bd7924` прошли; результаты применённого
frontend-кода проверяются отдельно в Issue #31. Новых folder controls, writer UI
или установки приложения этот объём не добавляет. Другие native-потребители и
полный read/write пользовательский маршрут ещё требуют завершения этапа 4.

## Инспектор, измерение выбранного тела и пары

App передаёт binding исходного открытия в SelectionModelProperties. Инспектор
использует FolderSourceRef в существующем inspect_model; его query cache учитывает
opening/source/task group/commit дополнительно к project/revision. Cache и поздний
ответ предыдущего открытия с одинаковыми IDs не подставляются в новое. Явно чужой
или отсутствующий binding отключает exact query, сохраняя mesh estimate без
managed fallback. Смена metadata-commit перезапускает строгую inspection-проверку.

Измерения выбранного тела и двух topology references получают binding вместе с
исходным геометрическим контекстом. Identity включает opening/source/task group,
revision/body/query, исходный program и SHA/size основного STEP; для пары — также
scene token и захваченные authored references. Нативные API используют ссылку самого
capture; другой source/commit/group нельзя подставить явным override. После смены
открытия отчёт, выбор пары и устаревшие callbacks capture/clear не принимаются.

Metadata-only refresh того же открытия сохраняет геометрически совместимый report
и выбор. Capture работающего запроса остаётся прежним; session обновляет только
контекст следующего запроса до свежей commit-ссылки. Это не recapture другого
источника: backend независимо проверяет исходный RepositoryRead, authored bindings
и реальные STEP assets. В отличие от строгого preview AI-review, совместимые
измерения не требуют сбрасывать результат только из-за имени или сообщений.

Folder-состояния раскрытия этих панелей живут в RAM через source preferences,
не читают и не пишут managed preferences и не сохраняют process-local tokens.
Прямые legacy callers могут опустить binding; production UI передаёт его явно.
Состав 16 файлов и проверки — в [контракте измерений renderer](../integration/storage-measurement-renderer-source.md).
Face/manual sections, history/import/export и writer/permission UI остаются
следующими интеграциями; видимые folder controls пока не добавлены.
### Snap исходной отображаемой сцены

Pointer и контекстное меню используют read-контекст действительно загруженного
объекта. Контекст захватывается до загрузки и хранится в WeakMap без записи в
GLB/JSON. Запрос передаёт исходный FolderSourceRef; поздние ответы после смены
открытия, геометрии или нового размещения курсора не меняют координаты и ошибки.
Metadata refresh сохраняет совместимый запрос с его исходным commit; следующий
запрос получает новый commit. Старое меню не переносит курсор новой папки.
Отказ host о stale snapshot не заменяется оценочной точкой. Обычная ошибка worker
может дать явно non-exact оценку только в исходном действующем контексте.
Точный объём и проверки: [контракт snap](../integration/storage-snap-renderer-source.md).

### Одноразовое решение host для исходной команды

Rust-модуль `window_sources/approval` сохраняет исходный `FolderOperation` и
типизированную команду Model/History до решения пользователя. `request_write`
проверяет окно, source ID, project ID и ожидаемый commit. `resolve_write` принимает
решение только для этого окна; `consume_write` требует ту же исходную связку.
Выдаваемый `ApprovedCommand.execute` выполняет сохранённую команду без подмены
payload после подтверждения. Проверку CAD-программы выполняет общий executor;
host обязан проверять входные размеры до создания запроса из IPC.

Запросы живут в RAM не более пяти минут, максимум 128 одновременно; перезапуск,
закрытие источника или окна не восстанавливает разрешение. Отказ, повторное
подтверждение, повторное использование, другая папка с тем же ID и изменение
сохранённого состояния не дают право записи. ReadSession и audit/settings не
являются разрешением. Проверка истории и реальный OS writer lease остаются
обязательными в существующем publisher. Сам запрос ничего не пишет в проект.

Общий registry mutex не удерживается через await; отдельный mutex сериализует
решение и использование одного запроса. Девять регрессий используют настоящие
папки FCP1/SQLite: сохранение исходной команды, отсутствие записи при согласовании,
чужие selectors, отказ/expiry/replay, close/reopen, новый commit, конкурирующее
использование, независимые запросы и предел registry. CI запускает их вместе с
существующими window-source тестами. До успешного CI новые Rust-проверки не считаются
выполненными локально: на этом ПК нет полного Windows SDK.

`request_prepared_write` передаёт исходный Project чистой host-подготовке команды
после проверки выбранной связки. Это позволяет существующим permission/command IPC
использовать общий валидатор без повторного захвата источника. Отказ подготовки
не создаёт pending request. Неверный commit не запускает даже callback подготовки.

Собственные Storage comparison/native workflows сохраняют работающий запуск при
следующем push и ставят следующий в очередь (`cancel-in-progress: false`). Проверки
не сокращаются; незапущенный промежуточный SHA не объявляется проверенным. Общий
Quality workflow не меняется. Результаты всегда относятся к точному SHA запуска.

Отдельный явный native CI-тест выполняет сохранённые CAD-команды 40→80 мм,
затем Undo/Redo через тот же одноразовый маршрут и читает реальные STEP файлы
worker для проверки объёма и габаритов. Он требует явно собранный worker и
запускается с `--ignored --exact`; обычный skipped/ignored запуск его не доказывает.

Это host-основа, пока без нового пользовательского действия. Общие
`request_permission`/`resolve_permission` и model/history IPC ещё не подключены к
этому маршруту: для них нужен отдельный согласованный патч. Дублирующего IPC,
записей managed project и сериализуемых grants не добавлено. Кнопка папки остаётся
скрыта до готовности полного read/write сценария.

### Ручное сечение исходного STEP

Существующий `section_model` принимает необязательную пару sourceId/expectedCommitId.
Host проверяет исходное окно и передаёт RepositoryRead выбранной папки в тот же CAD
worker; при неверной или неполной паре managed fallback отсутствует. Чтение сырого
STEP не требует authored program. До и после worker, включая очистку workspace,
строго проверяются исходный снимок, revision и sealed bytes. Metadata-only commit
тоже инвалидирует ручное сечение. Проверенные байты передаются общему stager без
второй полной staging-копии.

Согласованы и применены шесть файлов из [точного контракта](../integration/storage-manual-section-source.md).
Production build и rustfmt прошли локально. Новый настоящий FCP1/STEP тест проверяет
периметр 120 мм при повреждённом managed-двойнике и отказ после закрытия источника;
его явный native CI PASS подтверждён на fcd7ab1 (native42226643167). Frontend API и production caller обоих
режимов сечений подключены после отдельного согласования девяти файлов из
[renderer-контракта](../integration/storage-section-renderer-source.md).

Ручное сечение использует исходный commit, скрывает поздние result/error после
смены открытия и перезапускается после metadata commit. Сечение выбранной грани
сохраняет геометрически совместимый результат при metadata refresh; следующий запрос
получает свежую ссылку. Ни один запрос не перенацеливается на другую папку. Оба режима
сохраняют порядок одного незавершённого native вызова.

Контекст реального загруженного объекта берётся из WeakMap. Удерживаемая геометрия
сохраняет локальную GPU-плоскость только в том же открытии; точные кривые ожидают
актуальную отображаемую сцену. Folder-объект без исходной привязки не получает
managed fallback. Настройки панели в папке хранятся в RAM и сбрасываются при новом
открытии, без чтения/записи managed preferences. Полный folder writer UI всё ещё
не готов; кнопка папки остаётся скрытой.
