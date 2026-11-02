# Draft: verified V2-to-FCP1 migration / Проверяемый перенос V2 в FCP1

Status: implemented in the task branch under verification, with no runtime PASS yet. Inspected design base
`ae1b0ab` and §10.11 of the assigned storage specification. This does not start
stage 5, authorize shared wiring, or declare production FCP1 available.

Review corrections: backup retains the destination identity captured before
creation through capability mint; migration retains target identity through
post-commit verification and compares the reopened target with the exact saved
commit receipt. Linux regressions substitute identical-payload directories and
databases, which must remain indeterminate with the original commit UUID.
The same post-publication verifier is used by Save Copy: retained target identity,
exact commit receipt, validated SQL/history and all referenced CAS. Reopening a
new folder is never a way to refresh the original target authority.

Native CI at `8b08238` passed 488 library tests and failed one migration test:
the target serializes the validated archive in canonical struct field order,
while the source fixture uses a different JSON object order. Verification now
validates both full archives and compares their complete JSON values, including
all ordered events and cursor state. Raw backup bytes remain exact; conversion
does not promise identical JSON formatting. The correction requires fresh CI.

Исправления ревью: identity backup захватывается до создания и сохраняется до
выдачи capability; identity назначения удерживается через финальную проверку,
а открытый target сравнивается с точным сохранённым commit receipt. Linux-тесты
подменяют каталоги и БД с одинаковыми данными; исходный commit UUID должен
сохраняться как неопределённый результат. На `8b08238` native CI прошёл 488
library-тестов и нашёл один отказ миграции из-за различного порядка JSON-полей
архива. Теперь проверяются оба полных архива и сравниваются их полные JSON-значения,
включая порядок events и cursor. Байты backup остаются точными, форматирование
JSON после конверсии не обещается. Исправления требуют нового CI.
Общая финальная проверка используется также Save Copy: исходная identity
назначения, точный commit receipt, SQL/history и все используемые CAS. Новое
открытие папки не обновляет право принять результат старой операции.

## RU

### Что переносится

Источник — явно открытая папка с опубликованным manifest storageVersion 2,
проверенная существующим `FolderRepository` и его исходным
`FolderReadSnapshot`. Назначение — новая самостоятельная FCP1-папка. Никакого
изменения формата исходника на месте, reconcile, app SQLite, установки или
удаления старого каталога нет. Миграция сохраняет project UUID, revision UUID,
содержимое Project, курсор и переносимый архив истории; repository/commit UUID
нового FCP1 создаются заново. Это отличается от `save_copy`, который назначает
новый project UUID и новые timestamps.

Конверсия сохраняет действующие пределы FCP1: 40 МиБ на вложение и 100 МиБ
суммарных ссылок файлов кандидата. Допустимое V2-поколение, превышающее эти
пределы, получает отказ до создания target; подтверждённый backup сохраняется.
Не выполнять скрытое усечение, удаление вложений или расширение лимитов.

Неизвестный storageVersion отклоняется существующим manifest validator до записи
backup/target. Отсутствующий либо непроверяемый manifest не трактуется как V2.
Для заявленного переноса истории требуется проверенный history archive; если
архив отсутствует, API возвращает явный неподдержанный вход, а не синтезирует
историю из последней ревизии. Пустой исходный проект тоже переносится только
по отдельно определённому контракту архива, без скрытого изменения правила.

Настоящий проект 1.1 не передаётся этому API. Явный перенос одного последнего
STEP — отдельный импорт в новый project UUID с честно отсутствующей
восстановленной параметрикой/историей; наличие STEP не разрешает объявить старый
каталог поддерживаемым V2.

### Резервная копия до преобразования

Backup — отдельный явно выбранный пустой каталог, содержащий **точные байты
опубликованного manifest и всех перечисленных им файлов текущего поколения**.
Он остаётся читаемым V2. Это backup committed snapshot, а не рекурсивная копия
всего source tree: неперечисленные файлы, старые поколения, lock hints и чужие
настройки не копируются. Сохраняются исходные whitespace и представление допустимого
project JSON. Неизвестные поля Project отклоняет существующий строгий decoder;
перенос не ослабляет его и не удаляет такие поля молча.
Каноническая повторная сериализация Project не называется byte-exact backup.

До создания target FCP1 необходимо:

1. Проверить исходный lifecycle/location/manifest и все допустимые размеры,
   выбрать отличающиеся source/backup/target расположения. Ни одно назначение
   не должно быть source, его предком/потомком или другим назначением. Новый
   каталог резервируется create-new операциями и реальной cooperative lease;
   lease не является эксклюзивным владением namespace против постороннего ПО.
2. По одному читать только manifest-listed entries через bounded hash-checked
   reader. Файлы backup создаются исключительно новыми; существующая запись
   вызывает отказ, а не перезапись/доверие по одному имени. Не использовать
   generic `immutable_write` AlreadyExists reuse для этого нового протокола.
   Проверять retained destination directory identity и original source binding
   перед/после существенных IO. Буфер не больше действующего per-entry лимита.
3. Flush новых файлов и необходимых каталогов по доступной платформенной модели.
   Опубликовать точные manifest bytes последними через exclusive create;
   сохранить известные manifest SHA-256/generation даже при поздней ошибке.
4. Открыть backup новым `FolderRepository`, снова проверить exact manifest seal,
   Project, архив и **каждый** manifest-listed asset, включая валидные entries,
   не выбранные Project.files. Сравнить все entry size/hash и source seals.
   Один факт наличия backup directory или manifest не является проверкой.
5. Только после успешного reread выдать непрозрачный `VerifiedV2Backup` с
   retained backup opening/snapshot/location и known manifest seal/generation.
   Его поля закрыты, он не десериализуется из UI или настройки.

Пока backup не подтверждён, target не получает SQL или CAS. Прерванный backup
может оставить собственные неполные файлы; автоматического clean/replay нет.

### Конверсия после проверки backup

Предпочтительный API состоит из двух явных шагов:

```rust
FolderRepository::backup_committed(
    &self, original: &FolderReadSnapshot, empty_backup: &Path
) -> Result<VerifiedV2Backup>;

async fn migrate_verified_v2(
    source: &FolderRepository,
    original: &FolderReadSnapshot,
    backup: &VerifiedV2Backup,
    empty_target: &Path,
) -> MigrationReport;
```

`VerifiedV2Backup` и raw entry backup принадлежат ясному owned модулю, например
`project_storage_v2/reader/backup.rs`; lifecycle wrapper — небольшому
`project_repository/folder/backup.rs`. Конверсия —
`project_repository/sqlite_folder/migration.rs` с отдельными tests. Это новые
ответственности, а не вторые CAD/SQL commit алгоритмы.

Конверсия проверяет первоначальный source token и backup capability; читает
типизированный Project, проверенную историю и уникальные CAS-вложения из
подтверждённого backup; выполняет существующий
`creation::create_checked(..., before_sql)` с сохранённым Project без rename,
новых timestamps или project UUID. `before_sql` повторно проверяет исходный
source и backup после CAS staging. После commit — проверка обоих источников и
новое чтение target через FolderSession, полное сравнение Project/history и
хеш-проверка всех его используемых CAS. Результат не скрывает фактически
состоявшийся commit при позднем отказе.

`MigrationReport` должен сохранять обе уже созданные сущности: backup
seal/generation/location и target outcome с известным commit UUID. Различать
отказ до target, отказ SQL до COMMIT, подтверждённый target и неопределённый
target. Обычный `Result<Project>` теряет факт готового backup при ошибке target
и поэтому недостаточен. При ошибке после target COMMIT нельзя вернуть
«откат», повторить конверсию или удалить target автоматически. Сначала
разрешается known UUID через самостоятельный target reader.

### Минимальные API и существующие ограничения

Публичные `FolderRepository`/`FolderReadSnapshot` сегодня дают Project,
выбранные artifacts и optional history. Они **не дают** исходные manifest bytes,
generation, seal или доступ ко всем manifest-listed entries. Поэтому с ними
нельзя доказать raw V2 backup без нового owned reader extension. Нельзя
восполнить это обходом `read_dir` всего исходника: такой обход захватывает
непроверенные файлы и теряет captured-source authority.

Минимальное расширение reader — приватный/`pub(crate)` committed-entry exporter
либо метод backup, использующий уже существующий `read_entry` и exact sealed
manifest read. Нужен полный reread verifier всех captured manifest entries.
Metadata getters для UI, новая IPC и изменения native/Cargo не нужны.
Writer destination должен применять pinned/no-follow/create-new примитивы;
если пригодные платформенные примитивы уже есть в FCP1 CAS, выделять только
общую безопасную файловую ответственность, не копировать VFS/SQL алгоритм.

V2 pending mirror intent находится в app SQL `project_storage_commits`.
Самостоятельная V2-папка не содержит достаточных данных, чтобы определить,
есть ли в забытом внешнем app index такой pending intent. Поэтому этот API
переносит только явно выбранное **опубликованное поколение** и не обещает
обнаружить скрытый managed recovery. Для маршрута из managed проекта host
сначала обязан получить существующий pure LegacyManaged snapshot с проверкой
pending recovery либо предоставить отдельный согласованный proof eligibility.
Не выдумывать новый файл `pending.json` и не делать reconcile из миграции.

### Проверки, которые доказывают реализацию

- Source V2 с реальной сохранённой undo/redo историей, повторяющимися CAS hashes,
  дополнительным валидным manifest entry и необычным whitespace допустимого JSON
  полем: backup bytes/seal совпадают, target Project/history/revisions совпадают,
  original source tree побайтово неизменен. Новый процесс читает backup V2 и
  target FCP1, выполняет target undo/redo без app index.
- Target SQL/CAS отсутствуют, пока полный backup reread не подтверждён. Fault
  hooks после отдельных backup writes, после manifest и до verification
  отказывают; повреждённый backup никогда не превращается в capability.
- Future manifest version, отсутствие/повреждение history, malformed entry,
  oversized metadata/asset, traversal/link/reparse, source close/rename/reopen,
  backup/target непустые, совпадающие/вложенные корни: отказ без записи source
  и без target commit. Внешние/старые поколения не читаются и не копируются.
- Изменение source или backup после проверки и во время destination CAS
  staging: до SQL — отказ с сохранением backup provenance; после target commit
  — indeterminate с UUID. Не подменять original snapshot свежим capture.
- Отказ target creation после verified backup сохраняет backup receipt и файлы.
  Отмена не освобождает destination lease, пока detached IO ещё исполняется;
  не производится автоматического удаления и replay.
- Managed host pending recovery проверяется существующим pure legacy route;
  standalone test не выдаётся за проверку отсутствующего app SQL intent.
- Fixture 1.1 без V2 manifest остаётся побайтово прежним. Отдельный STEP import,
  если реализован, подтверждает новый ID и отсутствие обещанной параметрики.

Это набор требуемых тестов, а не результаты. USB/NAS и power-loss не проверены.

## EN

The proposed migration converts one explicitly selected, published storage V2
generation into a new FCP1 directory. It preserves the project/revision IDs,
Project payload and validated portable history; new repository/commit IDs belong
to the destination. Unlike Save Copy, migration does not rename the project or
change timestamps/project identity. The source is never modified or reconciled.
Unknown versions, absent/corrupt manifests and missing required history are
refused rather than treated as old parametric projects. A genuine 1.1 folder is
outside this API; importing its last STEP is a separate new-project operation
with no claim to recovered parametric history.

Conversion retains existing FCP1 limits: 40 MiB per attachment and 100 MiB of
candidate file references. A valid V2 generation above those limits is refused
before target creation, retaining its verified backup. No truncation, dropped
attachments or hidden limit expansion is implied.

Create and independently verify a separate **V2 committed-snapshot backup before
conversion**. Preserve exact sealed manifest bytes and every manifest-listed
entry, including raw whitespace and representation of supported Project JSON.
Unknown Project fields are refused by the existing strict decoder rather than
silently dropped or accepted by weakening validation. Do not recursively copy
unlisted files, old generations, lock hints or application settings. Read bounded
entries through the existing hash validator; create only new destination files,
retain location/lease checks, flush, and publish manifest last. Reopen the backup
and verify its exact seal and every entry, not merely Project.files or directory
existence. Only then mint a private, nonserializable `VerifiedV2Backup` capability.

Current public folder APIs expose typed Project, selected assets and optional
history; they do not expose exact manifest bytes/seal/generation or every entry.
An owned reader backup/export extension and complete reread verifier are therefore
necessary for a raw backup. A normalized reserialization is not a byte-exact
backup. Do not reuse `immutable_write`'s AlreadyExists branch for new backup files.
New owned modules can separate reader backup, repository lifecycle wrapper and
FCP1 conversion without adding IPC or changing native/Cargo.

After backup verification, read bounded Project/history/CAS from that retained
backup and call existing `creation::create_checked`. Recheck the original source
and backup after CAS staging and before SQL, then recheck and reopen the target
after commit. There is no transaction spanning three independent locations.
The typed migration report must retain verified backup provenance and every known
destination commit UUID on later failure. A late failure is indeterminate, never
rollback, permission to replay, or permission to delete a published target.

V2 pending mirror intent exists only in global app SQL, `project_storage_commits`.
The independent folder cannot discover a hidden intent in an unrelated forgotten
index. This API migrates the explicitly selected published generation and must
state that limitation. Managed callers need the existing pure LegacyManaged
pending-recovery check or a separately agreed eligibility proof. Migration must
not invent a pending-file protocol or run reconcile.

Required tests include independent-process backup/target reopen and target
undo/redo without app index; exact source purity; backup all-entry/seal integrity;
no target publication before verified backup; corrupt/future/missing-history
refusal; close/reopen/path replacement and nested/nonempty destination refusal;
post-staging source/backup conflicts; known-UUID preservation after commit; and
cancelled owned IO retaining its real lease. Real 1.1 remains unchanged. These
tests and hardware gates are proposed requirements, not verified results.
