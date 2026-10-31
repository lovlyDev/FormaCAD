# Решение о формате самостоятельной папки

[Документация](index.md) · [English](../en/storage-format-decision.md) · [Измерения](storage-formats.md)

Решение wenlixxq для дальнейшей реализации: **project-local SQLite с DELETE
rollback journal и FULL synchronous, плюс immutable CAS вложений**. Приёмку
программы и изменение общего маршрута приложения принимает lovlyDev. Это выбор
целевого backend, а не объявление уже доступного пользовательского сценария.
LegacyManaged и поддержанное чтение manifest v2 сохраняются; существующие папки
автоматически не переписываются. Benchmark schema не становится production schema.

## Основания

Изолированный SQL-прототип использует настоящие Project и типизированные
project/history правила без общей базы приложения. Выполнены сравнения
[de73580, 4f8b394 и d08a99e](storage-formats.md): metadata, history status,
ограниченное вложение, рост, независимые копии, настоящий CADPACK, новый процесс
и четыре process-death границы. Raw данные и окружение сохранены рядом с отчётом.
Повторные ссылки действительно переиспользуют один CAS-объект.

Измерения показывают меньшие затраты SQL-прототипа на этих фикстурах, однако
операции выполняют разную проверку. Основание выбора — также единая транзакция
для project payload, cursor и событий. Она устраняет необходимость согласовывать
отдельные manifest и cursor pointers и позволяет получать undo/redo без чтения
всех вложений и полного архива. CAS по-прежнему требует собственного протокола.

Альтернатива manifest/generations с отдельно запечатанным cursor возможна.
Текущая v2 такого entry не поддерживает: нужно изменение схемы, проверка
cursor/archive consistency и повторное сравнение. Это не измеренный вариант,
поэтому ему не приписываются скорости существующего metadata reader. Смена
формата не исправляет автоматически найденные CAS и legacy recovery дефекты.

## Обязательный контракт реализации

1. `project.sqlite` — authority проекта, модели и истории; глобальный app index
   хранит только производные location/recent/pinned/settings. Потеря индекса не
   требует восстановления модели из прежней общей SQLite.
2. Production format имеет собственные marker, application_id, schema version и
   project-local migration lineage. Значения benchmark не переиспользуются молча.
   Неизвестная версия или несовместимый schema отказывают до записи.
3. Reader открывает только проверенный поддержанный источник, без миграций,
   recovery, checkpoint, grants и индекса. Pending journal/WAL/SHM вызывает
   отдельное recovery-required/unsupported состояние; `immutable` не применяется.
   Все поля и события ограничены до materialization; синтетический admission с
   коэффициентом 6 не является окончательным production контрактом.
4. Snapshot удерживает backend/location/opening/epoch и исходное сохранённое
   состояние. Два источника с одинаковым project ID не взаимозаменяемы. Writer
   lease отдельна от read-session; право записи не выводится из флага UI.
5. Writer публикует проверенные CAS до SQL commit: собственный temp, bounded
   read-back, hash, flush и установка без усечения/перезаписи существующего CAS.
   Только после этого SQL transaction публикует metadata, cursor, events и новый
   commit ID с проверкой expected base/revision. Orphan CAS не является commit.
6. После SQL commit ошибка производного индекса не объявляет откат модели.
   Ответ различает «не committed» и «committed, индекс требует обновления».
   Отмена не удаляет чужие staging, CAS или lock-файлы.
7. Recovery вызывается явно под действительной writer/storage lease и заново
   проверяет location, expected commit и поддержанный формат. Чистый reader
   никогда не открывается write-capable ради автоматического SQLite rollback.
8. Нужен реальный model commit через согласованный CAD pipeline, затем новый
   процесс и undo/redo без app DB. Изменение только имени проекта в benchmark
   этим сценарием не считается.

## Совместимость и интеграционные gates

Первый безопасный перенос поддержанного LegacyManaged/v2 — новая проверенная
копия с новой identity и неизменным исходником. In-place migration потребует
отдельного crash/recovery протокола и backup до записи. Для настоящего 1.1
допустим перенос проверенного STEP в новый проект без выдуманной параметрики.

Общий open/save/copy/history, IPC, frontend и отображение ошибок подключаются
только согласованным точным патчем. Это решение не расширяет уже разрешённые
шесть measurement и три CADPACK test файла на остальные общие точки. Версии
приложения, Cargo/lockfile, выпуск и установка остаются у назначенного интегратора.

Первый production gate — проверенные локальные Windows/Linux/macOS filesystems.
Реальные USB/NAS, FAT/exFAT, сетевые locks и power loss пока не подтверждены;
их нельзя объявить поддержанными на основании tempdir CI. Конкретная матрица
носителей должна быть согласована до пользовательского обещания поддержки.

Профильные проверки этапа 2 выполнены на `d089e6c`; ссылки и реальные
результаты приведены в [программе](storage-program.md) и Issue №29. Приёмка
формата и production-интеграция остаются у lovlyDev.
Security CI отдельно сообщает уязвимости существующей Wasmtime в области
Issue #3; эта задачная ветка не меняет зависимость и не подавляет проверку.

## Чистая подготовка сохранённой модели

Первый согласованный шаг этапа 4 отделяет `modeling/prepared_change` от managed persistence. Текущий маршрут сохранения: настоящий CAD build → `commit::commit` → чистый `prepare` → `PreparedModelChange::into_parts` → прежний `projects::persist`. Preparer создаёт следующую ревизию и STEP/GLB descriptors, нормализует pending bytes, проверяет отмену и валидирует Project. Он не читает диск/SQL/clock и не публикует файлы. Host передаёт один timestamp; его получение перенесено непосредственно перед preparer, немного раньше прежней точки после data URL.

Имена файлов, parent/job/program binding, defaults параметров и metadata сохраняются. Идентичные output bytes получают одинаковые хеши; прежний normalizer может вернуть два pending элемента с одним хешем. Это не новая CAS-публикация или дедупликация. Ошибка normalization по-прежнему опережает cancellation; недопустимый Project теперь отклоняется до успешного кандидата, а не только внутри persistence.

Кандидат имеет закрытые поля и конструктор, не сериализуется и не является writer lease, permission, историческим intent или доказательством commit. Следующий backend должен проверить исходный сеанс/location/expected commit и историю под настоящим writer-доступом. Самостоятельная папка и общий asset/permission контракт этим шагом не реализуются. Шесть синтетических unit-регрессий подготовлены для metadata/revision preservation, blob descriptors, одинаковых хешей, отмены и ошибок; исполнение Rust-тестов требует нового CI. Синтетические bytes не доказывают геометрию CAD.
