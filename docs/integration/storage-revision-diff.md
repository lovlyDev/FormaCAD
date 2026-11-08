# Чистое сравнение сохранённых ревизий / Pure saved-revision comparison

## Русский

Статус: **lovlyDev разрешил два файла; патч применён, все шесть регрессий прошли на d089e6c**.
Автор предложения — wenlixxq. Владелец общей интеграции и приёмки — lovlyDev.
[Патч](storage-revision-diff.patch) подготовлен относительно
`a12064e7a6b32fe180b0dd1f773f6e9e689790f2` задачной ветки. Этот SHA содержит
используемый контракт `ProjectRepository` и проверяемые lifecycle-снимки.

Это следующий отдельный вертикальный маршрут этапа 3 после проверенного этапа 2,
а не разрешение менять все read consumers. ТЗ `friend-week-storage-spec.md`, §5,
требует для общих точек точного патча и согласования владельца в Issue. lovlyDev
отдельно разрешил эти два файла после подготовки предложения. Согласование шести файлов
измерений не распространяется на эти два файла.

**Точный scope:** изменить только `apps/desktop/src-tauri/src/cad_ir/history.rs`
и добавить `apps/desktop/src-tauri/src/cad_ir/history/session_tests.rs`.
Нет правок IPC-регистрации/сигнатуры, `projects::get`, frontend, Cargo, native,
worker, миграций, версий или выпуска. Пользовательские строки не добавляются:
сохраняются существующие ошибки и уже переведённые storage-коды.

Сейчас `compare_ir_revisions` вызывает `projects::get`, который может выполнять
`reconcile`. У сравнения нет удерживаемого сеанса и финальной проверки источника.
Глобально менять `get` нельзя: явное открытие и маршруты записи намеренно
используют его recovery. Предлагаемый патч меняет только чистое сравнение:

1. Создаёт явный managed `ProjectRepository` и один `RepositorySnapshot`.
2. Читает обе программы ревизий из этого же immutable snapshot, сохраняя проверки
   ID документа и существующую типизированную функцию `diff_documents`.
3. После вычисления diff либо legacy `None` выполняет `recheck` исходного snapshot,
   не захватывая новый token для принятия старого результата.
4. Pending recovery возвращает `PROJECT_SNAPSHOT_RECOVERY_REQUIRED`; читатель
   ничего не публикует, не очищает журнал и не получает writer lease.

Отсутствующая программа, legacy script и JSON без `schemaVersion` по-прежнему
дают `None`. Сохраняются сообщения `CAD revision is missing`,
`CAD IR revision does not match project` и прежнее `Project was not found`
для отсутствующего проекта; остальные parser/validation ошибки не подменяются.
Закрытие/reopen, смена source/location или сохранённого состояния возвращают
существующий snapshot-stale код. Никакие tokens не передаются из UI и не
сохраняются в проекте. Это optimistic recheck перед возвратом, не блокировка
последующего внешнего писателя. Структурный diff не доказывает изменение BREP.

Шесть предлагаемых fixture-тестов проверяют:

- точные две исходные ревизии, ожидаемый structural diff и неизменность SQL/файлов;
- close/reopen при тех же сохранённых байтах во время финального SQL recheck:
  пул реально исчерпывается, future опрашивается до `Pending`, без sleeps/hooks;
- отказ при pending recovery с неизменным исходным journal payload и sentinel;
- отказ от snapshot другого корня при одинаковых project/revision IDs;
- legacy `None` при read-only доступе, отсутствие upgrade даже после освобождения
  чужого OS lease, неизменность SQL/файлов;
- legacy script/JSON fallback и сохранение прежних ошибок отсутствующей ревизии,
  проекта и несовпадающего authored revision ID.

Фикстуры создают настоящую SQLite и типизированные synthetic CAD IR документы,
но не вызывают CAD worker. Сравниваются логические данные SQL, журнал/история и
файлы внутри проекта. Изменение информационного owner JSON при явном reopen не
выдаётся за изменение модели. Приватный `compare_captured` — реальный внутренний
маршрут production-вызова, а не отдельная тестовая реализация или глобальный setter.

При подготовке прошли только rustfmt временных копий и
`git apply --check docs/integration/storage-revision-diff.patch`. Рабочие shared
файлы затем изменены только после разрешения. Исполнение подтверждено позже:
[Quality 41137407828](https://github.com/lovlyDev/FormaCAD/actions/runs/41137407828)
на `d089e6c` прошло все шесть consumer regression tests, strict Clippy и
функциональные jobs четырёх платформ. Весь Quality имеет отдельный security failure.
Проверка применённого патча и исполнения:

```sh
git apply --reverse --check docs/integration/storage-revision-diff.patch
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib cad_ir::history::session_tests
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --all-targets --features native-worker-client -- -D warnings
npm run check:docs
npm run check:i18n
git diff --check
```

Исполнить также затронутые repository lifecycle и существующие CAD diff tests,
зафиксировать head SHA и реальные результаты в Issue/PR. RU/EN документацию
пользовательского маршрута после применения обновить из статуса предложения
в фактическое поведение. Это не перенос всего списка проектов, не folder write
backend и не готовность всех consumers этапа 3.

## English

Status: **lovlyDev approved both files; applied, all six regressions passed on d089e6c**. wenlixxq prepared
the [two-file patch](storage-revision-diff.patch) against
`a12064e7a6b32fe180b0dd1f773f6e9e689790f2`. lovlyDev owns integration and acceptance.
The storage specification §5 requires agreement on this concrete shared scope
before applying it. lovlyDev gave separate explicit permission for both files.
Prior permission for six measurement files was not treated as covering them.

Scope is only `cad_ir/history.rs` and new `cad_ir/history/session_tests.rs` under
`apps/desktop/src-tauri/src/`. IPC registration/signature, global `projects::get`,
frontend, Cargo/native/worker, migrations, versions and release remain unchanged.
The existing comparison calls `projects::get`, which may reconcile storage and
does not retain a read token. This proposal uses one explicit managed repository
snapshot for both saved revisions and rechecks that original authority after
computing either a diff or legacy `None`. It neither runs recovery nor acquires
write access. The global opener/writer route remains intact.

Missing programs, legacy scripts and JSON without `schemaVersion` retain `None`.
Existing missing-project/revision, authored-ID, parser and validation errors remain;
new stale/recovery outcomes use already supported storage codes. No new user text
is introduced. Rechecks are optimistic observations before return, not locks on
future writers. Structural changes are not BREP geometry claims.

Six fixture tests exercise successful original-revision comparison, deterministic
close/reopen during the actual blocked final SQL recheck, pure pending-recovery
refusal, identical IDs at different roots, legacy read-only behavior without
upgrade, and preserved fallback/error cases. Pool exhaustion and explicit polling
replace timing assumptions. The fixtures use real isolated SQLite and synthetic
typed IR; they do not need the CAD worker or prove installed UI acceptance.

Only temporary-copy rustfmt and patch applicability passed during preparation.
The patch was applied after explicit owner agreement. Subsequent
[Quality 41137407828](https://github.com/lovlyDev/FormaCAD/actions/runs/41137407828)
on `d089e6c` passed all six consumer tests, strict Clippy and four functional
platform jobs; the whole run has a separate security failure. Reproduce with the commands above and CAD diff
tests. Record the exact final SHA and passed/failed/skipped evidence in Issue/PR,
then update operational RU/EN documentation. This proposal does not migrate the
whole project list, provide folder writes, or complete stage 3 by itself.
