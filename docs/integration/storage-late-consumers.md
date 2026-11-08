# Поздние CAD-ответы: точки принятия / Late CAD result acceptance

## Русский

Статус: **lovlyDev явно разрешил восемь файлов; точный патч применён, исполнение ожидает CI**.
[Патч](storage-late-consumers.patch) основан на
`d089e6c016cb5e03efb9c6d391a2d475ec36b0fe`. Автор предложения — wenlixxq;
согласование общих точек, приёмка и слияние — lovlyDev. По §5 ТЗ
`friend-week-storage-spec.md` для shared-точек требуется «подготовь точный
контракт/патч, укажи причину и согласуй владельца в Issue». Ранее разрешённые
шесть файлов измерений не дают разрешения применить это предложение.

### Точный scope и независимые группы

Изменения ограничены восемью файлами под `apps/desktop/src-tauri/src/`:

Итого: **две правки production-поведения и шесть новых регрессий**. Отдельный
приватный seam ручного сечения нужен тесту; публичный API и обычный worker flow
от этого seam не меняются.

| Группа | Файлы | Поведение |
| --- | --- | --- |
| A: preview и свойства STEP | `model_preview/host.rs`, `model_preview/host/tests.rs`, `model_inspection/read/tests.rs` | Preview повторно проверяет исходник после cleanup-await; отдельные late close/reopen проверки preview и STEP properties |
| B: ручное сечение | `model_section/capture.rs`, `model_section/build.rs`, новый `model_section/build_tests.rs` | Полный recheck исходного снимка после sealed-asset IO; реальная worker-регрессия late close/reopen |
| C: pair/face | `model_pair_measurement/build_tests.rs`, `model_face_section/build_tests.rs` | Только новые consumer-specific late close/reopen тесты |

Группы независимы и могут согласовываться/передаваться отдельно. Файлы группы B
нужно переносить вместе: build регистрирует новый test module и его приватный
worker-boundary seam. В A изменения preview source и его tests также связаны.
Общий `.patch` перечисляет ровно указанные файлы; он не даёт полномочий на соседние
модули. Изменений C++/OCCT, Cargo, worker protocol, IPC, frontend, глобального
`projects::get`, миграций, версий и выпусков нет.

### Закрываемые точки принятия

Preview уже проверяет исходный `CommittedSnapshot` после worker, но затем
ожидает `state.tasks.lock()` для удаления маркеров. За это время source/session
может смениться. Предложенный приватный `complete` — реальная завершающая часть
production `run`: очищает маркеры в отдельном mutex-scope, отпускает mutex,
сохраняет прежнюю ошибку worker при неуспехе и только для успешного packet
выполняет полный recheck **того же** снимка и cancellation check перед возвратом.
SQL/файловая проверка не удерживает mutex task map. Новый token не захватывается
для легализации старого ответа.

Ручное сечение сейчас проверяет снимок перед чтением оригинального STEP. Вторая
полная проверка добавляется **после** size/hash-проверки прочитанных bytes.
Она использует исходный snapshot и прежний `snapshot_error` mapper. Проверки
expected revision, SHA/размера источника, stage/import, отчёта worker и отмены
остаются. Это более поздняя optimistic observation, а не OS-блокировка внешнего
писателя. Дополнительный полный recheck имеет стоимость: оценить её на больших
STEP при исполнении, не объявлять производительность проверенной по rustfmt.

`model_section/build.rs` сохраняет публичную сигнатуру. Добавленный приватный
`section_inner` используется production wrapper; его `PhaseHook` имеет поле
`after_worker` только при `cfg(test)`, по образцу существующих измерительных
тестов. Он вызывается только после настоящего проверенного worker report, не
попадает в IPC и не позволяет подать выдуманную геометрию.

Production-маршруты pair/face measurement не меняются. В частности, queued
metadata-only save по-прежнему разрешён: исходный lifecycle удерживается отдельно
от сравнения revision/body/document/source/import/preview seals. Предложение не
возвращает для них полный equality-check всего старого Project payload.
Preview и manual section сохраняют свою прежнюю строгую модель saved snapshot.
Новые пользовательские строки не добавляются; используются существующие stale,
recovery и cancellation сообщения с прежним переводом на границе UI.

### Реальные проверки, предлагаемые патчем

Добавлены шесть регрессий:

1. Свойства STEP: настоящий worker возвращает объём известной коробки 8000 мм³,
   затем source закрывается/открывается без изменения данных; старые properties
   отклоняются. Новый запрос проходит, SQL/история и файлы проекта прежние.
2. Preview: настоящий worker создаёт проверенный GLTF packet, затем close/reopen
   отклоняет старый ответ; маркеры/queue очищены, свежий запрос возвращает GLTF.
3. Финальный preview cleanup: после настоящего GLTF и precheck вызывается тот же
   production `complete`; реальный task-map mutex удерживается, future опрашивается
   до `Pending`, выполняется close/reopen, mutex освобождается. Packet отклоняется,
   а обычные request markers удалены. Sleeps и глобальных phase setters нет.
4. Ручное сечение: callback выполняется после реального проверенного section
   report, close/reopen вызывает `SECTION_STALE`; свежий запрос даёт периметр
   коробки 120 мм на выбранной плоскости и исходный SHA.
5. Pair measurement: существующий private after-worker seam выполняет close/reopen;
   старый report отклоняется, свежий проходит, сохранённые данные неизменны.
6. Face section: то же для своего настоящего section worker/report и отдельного
   transient workspace.

Проверки не подменяют worker метриками. В каждой late-worker регрессии должен
действительно завершиться соответствующий проверяемый native вызов; ошибки
раньше этой точки не считаются доказательством. Сравниваются данные SQL/истории
и файлы проекта; informational owner JSON при явном reopen не считается моделью.
Все workers и временные каталоги принадлежат обычным изолированным fixtures.
Не завершаются чужие процессы и не удаляются пользовательские проекты.

Эти регрессии не доказывают unplug USB/NAS, установленный WebView или атомарность
между финальным recheck и дальнейшим внешним writer. Отдельная межпоточная
fault-injection регрессия закрытия именно посреди synchronous STEP asset read
не добавляется: после-IO fence здесь проверен только по структуре патча и должен
быть учтён в дополнительной приёмке при наличии такого стенда.

### Подготовленная проверка и применение после согласования

Пока прошли только rustfmt временных копий и
`git apply --check docs/integration/storage-late-consumers.patch`.
После отдельного разрешения lovlyDev shared-исходники изменены точным патчем. Compile/Clippy/worker PASS для этого изменения
не заявлен; подтверждённая основа не заменяет исполнение нового патча.

Проверка применённого патча и его исполнения:

```sh
git apply --reverse --check docs/integration/storage-late-consumers.patch
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --all-targets --features native-worker-client -- -D warnings
```

Следующим командам нужен подготовленный native toolchain и реально собранный
совместимый `forma-cad-worker` с его runtime по штатному пути fixtures:

```sh
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-occt --lib model_inspection::read::tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-occt --lib model_preview::host::tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-occt --lib model_section::build::tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-occt --lib model_pair_measurement::build::tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-occt --lib model_face_section::build::tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features native-occt --test reference_measurements_host --test pair_measurements_host --test face_sections_host --test model_section_projects
npm run check:docs
npm run check:i18n
git diff --check
```

Не пропускать metadata-only integration tests и имеющиеся corruption/cancellation
регрессии. Отсутствующий/пропущенный worker не считается PASS. Зафиксировать SHA,
feature set, passed/failed/skipped и реальные ограничения; после применения
обновить operational RU/EN карту потребителей и проверить прежние RU/EN stale
состояния UI. Это узкий этап 3, не завершение всей программы хранения.

## English

Status: **lovlyDev explicitly approved eight files; applied, execution awaits CI**. The
[patch](storage-late-consumers.patch) targets
`d089e6c016cb5e03efb9c6d391a2d475ec36b0fe`. wenlixxq prepared it; lovlyDev owns
shared integration and acceptance. Specification §5 requires agreement on these
specific files before application. Prior permission for six measurement files
does not cover this scope automatically.

The eight files and three independent review groups are listed above: A changes
preview completion and adds preview/inspection tests; B changes manual-section
capture, adds its private test seam and worker test; C adds only pair/face tests.
There are exactly two production behavior changes and six new regressions; the
manual-section seam is private test infrastructure, not a new public operation.
Keep each connected source/test group together. There are no Cargo, C++/OCCT,
protocol, IPC, frontend, global project opener, migration, version or release edits.

Preview completion releases the task-map mutex after cleanup and then rechecks
the original snapshot before accepting a successful packet. Prior worker errors
retain their meaning and cleanup still runs. Manual section adds a full original
snapshot check after sealed STEP reads, retaining existing revision, artifact,
report and cancellation checks. Its test seam is private and present only in test
builds, with no fabricated metrics or IPC authority. Additional full-read cost
requires real performance measurement rather than a formatting-based claim.

Production pair/face measurement behavior remains unchanged, including acceptance
of unrelated queued metadata-only saves and separate original-lifecycle versus
geometry-seal checks. Existing error translations remain; no user strings are added.

Six proposed real-worker regressions cover late close/reopen for STEP properties,
preview, manual section, pair measurement and face section, plus a deterministic
preview cleanup-await test using its real completion function and held mutex after
a genuine GLTF packet. They assert rejection, cleanup, unchanged stored data and
fresh retries. The STEP/property and manual-section retries also check known box
volume/perimeter and the section's source seal. No unrelated processes are killed
and no user data is cleaned. Mid-read concurrent STEP fault injection is not added;
that separate gate remains explicit despite the proposed post-IO code fence.

Only temporary-copy rustfmt and patch applicability passed. No compilation,
Clippy or worker execution is claimed. The patch was applied after explicit
owner agreement for all eight files. Run the
commands above with a real compatible worker and native environment, including
existing corruption, cancellation and metadata-only host suites. Record exact
SHA/features and passed/failed/skipped evidence. Missing/skipped native execution
is not PASS, and source checks remain optimistic rather than NAS/USB, installed
UI or future-writer guarantees.
