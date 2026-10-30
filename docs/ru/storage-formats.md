# Сравнение форматов хранения

[Документация](index.md) · [English](../en/storage-formats.md) · [Программа](storage-program.md)

Выбор самостоятельного folder backend ещё не принят. Сравниваются существующие
manifest/generations v2 и экспериментальная SQLite внутри папки проекта.
SQL-кандидат теперь использует минимальные project/history таблицы и типизированную историю; общая база
приложения не участвует в его синтетических данных. Это benchmark, а не доступный
пользователю open/save маршрут. Первый сохранённый baseline использовал app schema;
он сохраняется отдельно и не считается проверкой нового чистого opener.

Исправленный кандидат `sqlite-delete-prototype` использует rollback journal DELETE
и synchronous FULL. Явное create не переиспользует существующий файл. Reader
проверяет ограниченный SQLite header, application_id/user_version и отсутствие
WAL/SHM/journal до подключения, затем read_only/query_only и quick_check. Режим
immutable не используется. Reader не применяет миграций и не чинит журнал;
WAL purity остаётся неподдержанной. Это прототип benchmark, не готовый writer API.
Четыре регрессии проверяют неизменность файлов, отсутствие app-таблиц, запрет
записи и контролируемый отказ на чужом/будущем/WAL формате. Открытие writer
измеряется отдельно от сохранения metadata.
SQL copy/export preflight консервативно оценивает JSON escaping как шесть байт
на байт текста. Поэтому допустимый архив около лимита может быть отклонён раньше
общего лимита 16 МиБ. Сравнение использует одни small/medium/large фикстуры;
это ограничение прототипа, а не новый общий лимит пользовательской истории.

## Границы чтения

### Что требуется проверить до выбора формата

Аудит существующей v2 выявил две границы crash safety. Общий CAS сейчас
создаётся сразу под окончательным `assets/<hash>`: частичная запись может оставить
невалидный объект, который повторное staging отклонит. Старый опубликованный
manifest не делает такой объект новым commit. Будущее исправление должно сначала
записать и проверить собственный temp, затем установить окончательное имя без
перезаписи существующего CAS; reader не удаляет и не восстанавливает остатки.
Один `sync_all` файла не доказывает сохранность directory entry при потере питания.

В legacy recovery исходный manifest сравнивается до получения storage lock.
Перед публикацией требуется повторить сравнение под lock и очистить только тот
pending target, который действительно обработан. Иначе более новый внешний
commit может быть заменён старым. Эта находка не означает исправленный recovery.
SQL-кандидат тоже ещё не получает гарантию: его обычный opener отказывает при
journal; явное восстановление и аварийные точки требуют отдельного исполнения.
Выбор не может опираться только на более короткое время metadata save.

Новый opt-in crash benchmark завершает собственный дочерний test process
в четырёх точках: manifest staging/после publish и SQL transaction/после commit.
Для незавершённой SQL transaction требуется реальный hot-journal header и
изменённые страницы БД; reader обязан отказать без записи. Затем отдельно
вызывается SQLite recovery только на принадлежащей тесту фикстуре и проверяются
старые metadata, точный архив истории и CAS. После commit ожидается новое
metadata. Вывод разделяет время процесса, read/recovery и явного SQL recovery.
Проверка ожидает CI; это process death, а не обесточивание, production recovery
или свидетельство реального CAD commit/USB/NAS.

Новый `project_storage_v2::reader::FolderSnapshot` захватывает manifest и
project.json, проверяет ограниченные размеры, SHA и ссылки metadata. Он не читает
посторонние assets и архив событий. `read_asset` принимает только точный descriptor
из снимка и проверяет обычный файл, размер, SHA, исходный manifest и metadata.
`read_history` явно проверяет архив и его связи. `recheck` также обнаруживает
повреждение project.json при неизменённом manifest. Исходник не восстанавливается
и не изменяется. Отсутствующий asset не скрывается: metadata может быть прочитана,
но явное чтение повреждённого вложения завершается ошибкой.

Это низкоуровневый читатель сохранённых байтов. Repository adapter обязан отдельно
удерживать исходный lifecycle token и OS identity расположения. Существующий
`read_folder` сохраняет полную проверку всех записей. Существующая v2 хранит cursor
в полном архиве истории: benchmark не выдаёт чтение этого архива за O(1) status.

## Воспроизведение и свидетельства

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::benchmark::compare_storage_representations -- --ignored --nocapture`

Фикстуры small/medium/large содержат 4/32/128 сохранённых ревизий, переписку,
повторяющиеся и уникальные вложения. STEP-подписанные байты синтетические и не
являются проверенной геометрией. Применяются настоящие Project/история/manifest
валидаторы и лимиты. Метрики — сырые JSON-строки `STORAGE_BENCHMARK` без порогов.
Одна точка измерения не доказывает распределение задержек.

Отдельный workflow Storage comparison сохраняет benchmark.log, сведения CPU/OS/
диска и process-resources.txt. Последний включает компиляцию и весь процесс теста,
а не peak memory отдельной операции. Первый вызов API не означает холодный кеш
ОС. Новый тестовый маршрут запускает отдельный дочерний test executable и сверяет
PID, nonce, hash metadata, историю и один asset; время запуска процесса отделено
от операций reader. Кеш ОС остаётся неконтролируемым.
Прототип `verified_save_copy` создаёт новый UUID, переносит историю и вложения,
повторно проверяет исходник и читает результат. Источник не изменяется; отказ
может оставить собственный staging без опубликованного проекта. Тесты охватывают
повреждённый asset, изменение SQL-источника и превышение бюджетов до чтения.
Согласованный [тестовый hook CADPACK](../integration/storage-cadpack-benchmark.md)
измеряет настоящий writer, sync, reader и проверку содержимого как
`cadpack_round_trip`. Это временные синтетические назначения, а не production
copy/export transaction. Новые измерения ожидают исполнения CI; crash recovery,
холодный кеш и production publication не получают выдуманного времени.
Окончательные raw результаты и решение публикуются только после исполнения.

### Прогон изолированной SQLite

[Storage comparison 40945699832](https://github.com/lovlyDev/FormaCAD/actions/runs/40945699832)
и [Quality 40945699710](https://github.com/lovlyDev/FormaCAD/actions/runs/40945699710)
успешны для `de735801142dbdda56d9b5a0fe4f12cb0ad34681`;
checkout PR merge — `0454ce6d0a99d5a786f44ef232962bc7732e3331`.
Сохранены [сырые JSON](../storage-benchmark/de73580-linux.jsonl),
[окружение](../storage-benchmark/de73580-environment.txt) и
[ресурсы процесса](../storage-benchmark/de73580-process-resources.txt).
Ubuntu 22.04 / Linux Azure, AMD EPYC 9V45, 4 vCPU; виртуальный NVMe,
ROTA=0, физический тип не подтверждён. Rust 1.97.1. Для large однократное
metadata open: full manifest 313,520 мс, metadata reader 9,627 мс, SQL 4,013 мс;
history status: 628,322 / 18,342 / 0,782 мс соответственно. Metadata save:
manifest 1327,064 мс, SQL 8,852 мс плюс отдельно writer startup 1,523 мс.
Объём работы и проверки различаются, кеш не сбрасывался; это не выбор победителя.
Новые copy/CADPACK/child-process проверки добавлены позже и этим SHA не проверены.

### Первый выполненный прогон

[Storage comparison 40867790068](https://github.com/lovlyDev/FormaCAD/actions/runs/40867790068)
успешен для head `250016c6c9d69b12d7016bf5341a504603b685da`;
checkout PR merge — `ed96fbffbbb85f959b8db2abb8fa8d7cbba0a500`.
Сохранены [сырые JSON](../storage-benchmark/250016c-linux.jsonl),
[окружение](../storage-benchmark/250016c-environment.txt) и
[ресурсы процесса](../storage-benchmark/250016c-process-resources.txt).
Ubuntu 22.04 / Linux Azure, AMD EPYC 7763, 4 vCPU; диск виртуальный, `lsblk`
сообщает ROTA=1, физический тип накопителя не подтверждён. Rust 1.97.1.

| Large, один замер, мс | Полный manifest v2 | Metadata reader v2 | SQLite-кандидат |
| --- | ---: | ---: | ---: |
| Первый вызов API открытия | 526.348 | 16.335 | 7.772 |
| Статус истории | 1054.811 | 31.048 | 0.936 |
| Сохранение metadata | 2109.459 | Не отдельный writer | 14.922 |

Large: 128 ревизий, 256 ссылок на 128 уникальных assets по 128 КиБ,
архив 32355 байт, metadata 133911 байт. Повторное сохранение оставило 128 assets.
Эти операции делают разный объём проверки: SQL status не хеширует все assets и
архив; SQLite open использует имеющийся migration/recovery маршрут и ещё не pure
reader. Поэтому таблица показывает стоимость текущих примитивов, а не выбор
победителя с равными гарантиями. Cold-process/cache, полный copy/export, crash
recovery и peak memory отдельной операции остаются незамеренными. Прогон не
подтверждает более поздний head или весь этап 2.

Строгий CI на `147a0dd` остановился на Clippy explicit_auto_deref в benchmark
строке export_history; исходное падение сохранено в job 123427829406 того же
репозитория. Исправление `93956b1` убирает лишний deref, не ослабляя lint.

## Общая интеграция

Для измерений подготовлены [контракт и точный патч](../integration/storage-measurement-session.md)
на общей основе main. lovlyDev отдельно разрешил шесть перечисленных в нём файлов
29 октября 2026, патч применён в задачной ветке. Компиляция/настоящий worker ещё
проверяются. Остальная область Issue №3, протокол worker и Cargo не редактируются.
