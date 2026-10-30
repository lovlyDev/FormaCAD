# Доступ benchmark к настоящему CADPACK / Real CADPACK benchmark hook

## Русский

Статус: **lovlyDev разрешил три файла; патч применён в задачной ветке**. Приёмка и владелец общей
точки `cadpack.rs` — lovlyDev; предложение подготовил wenlixxq в рамках программы
хранения. [Патч](storage-cadpack-benchmark.patch) основан на `main` @
`4834e1020f755d4a158a105fc63965315d9c78c1`. Он не меняет Cargo, native, IPC,
пользовательские тексты, учётные данные, глобальный индекс или версии.

Причина: настоящие `write_bundle_with` и `read_bundle` закрыты внутри `cadpack`.
Существующий benchmark может измерять подготовку экспортных данных, но не
объявлять её измерением завершённого CADPACK. Новый encoder в тесте тоже не
проверил бы настоящий маршрут приложения.

Точный объём согласования: три строки регистрации `#[cfg(test)] pub(crate) mod
benchmark_api` в `cadpack.rs`, новый тестовый модуль `cadpack/benchmark_api.rs`
и его `benchmark_api_tests.rs`. Модуль доступен только unit-тестам той же
crate; production-сборка и внешние integration-test crates доступа не получают.
Он не открывает приватный `BundleContents` наружу.

Предлагаемая функция:

```rust
round_trip(
    project: &Project,
    destination: &Path,
    model_history: Option<&[u8]>,
    read_attachment: impl FnMut(&ProjectFile) -> Result<Vec<u8>>,
) -> Result<BundleProof>
```

Она вызывает существующие writer и reader. До callback проверяются валидность
Project, нормализованные файлы без embedded data, 2048 ссылок, 16 МиБ metadata,
16 МиБ истории, 40 МиБ на asset, 100 МиБ всех ссылок и 150 МиБ уникального
полезного содержимого; суммы складываются с проверкой переполнения. Окончательный
размер ZIP с overhead дополнительно проверяет прежний writer. Историю проверяет
прежний `validate_archive`. Callback обязан читать исходник с ограничением до
выделения буфера: hook передаёт исходный descriptor с размером, но не может
ограничить память произвольного Rust callback. Для сравнения использовать один
и тот же bounded reader/предварительно проверенный набор assets, без полной
дополнительной копии всех вложений на каждый шаг.

Writer сам dedup-ит хеши и проверяет bytes/hash. После записи реальный reader
проверяет пакет, ссылки и историю. Hook сравнивает точные metadata/history,
множество уникальных hash/size и число вызовов callback. Возвращаются только
хеши metadata/history, числа ссылок/уникальных assets/вызовов reader, объём
уникальных bytes и итоговый размер архива. Приватные типы и содержимое проекта
не становятся внешним контрактом.

Сеанс, provenance и исходный snapshot остаются ответственностью benchmark
driver: захватить согласованное состояние, ограниченно прочитать его данные,
проверить **исходный** token перед вызовом и после round-trip. При stale не
учитывать результат как успешный. Писать только в собственную синтетическую
tempdir, не в пользовательский экспорт. Hook не добавляет async/recovery и не
доказывает транзакционную публикацию пользовательского экспорта при смене
источника. Он также не доказывает save-copy, share-redaction, реальную CAD
геометрию, NAS/USB или новый backend записи.

В патч включены четыре проверки: реальный round-trip с историей и dedup,
неправильный hash сохраняет прежний destination и убирает собственный temp,
100 МиБ reference budget отклоняется до callback/записи, испорченная история
отклоняется до callback/публикации. Существующие CADPACK privacy/geometry/history
проверки остаются обязательными.

Проверено при подготовке: rustfmt временных копий и
`git apply --check docs/integration/storage-cadpack-benchmark.patch`.
Патч применён после явного разрешения lovlyDev. Форматирование проверено; компиляция и выполнение новых тестов ожидают CI. Команды воспроизведения:

```sh
git apply --reverse --check docs/integration/storage-cadpack-benchmark.patch
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --lib cadpack::benchmark_api::tests
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Hook подключён из owned benchmark; получить отдельные сырые результаты
для каждого сравниваемого представления на одном SHA. Строку назвать
`cadpack_round_trip` и явно включить в её стоимость write, sync, read-back и
валидацию; не выдавать её за чистое время записи. До реального выполнения
отсутствующий критерий приёмки copy/export не считается пройденным.

## English

Status: **proposed exact patch, not applied**. lovlyDev owns acceptance and the
shared `cadpack.rs` integration point; wenlixxq prepared the proposal. The
[three-file patch](storage-cadpack-benchmark.patch) targets `main` commit
`4834e1020f755d4a158a105fc63965315d9c78c1`. It adds only a `cfg(test)` crate-private
child module and focused tests, with no Cargo/native/IPC/credentials/index/version
changes. Private bundle types remain private and the production build gains no API.

The hook above invokes the real private writer and reader instead of implementing
another ZIP format. It validates normalized Project metadata and history, enforces
existing count, per-file, reference and total payload budgets before calling the
source callback, then verifies exact metadata/history, unique asset descriptors
and one callback per unique hash after the real reader succeeds. ZIP overhead is
still subject to the production writer's final archive-size check. The callback
must use a genuinely bounded source reader before allocation; a Rust callback's
internal allocation cannot be controlled by this test hook.

The plain proof contains only metadata/history hashes, reference/unique/callback
counts, unique asset bytes and archive size. The caller retains the original
repository snapshot, bounds reads, rechecks its original authority before and
after round-trip, and rejects stale observations. Use only disposable synthetic
destinations. This hook is not a production source-to-destination transaction,
save-copy backend, share-redaction route, CAD geometry or USB/NAS proof.

Four included regressions cover real history/asset round-trip and dedup, wrong
source hash preserving the previous destination, declared budget rejection
before source reads/writes, and malformed-history rejection before publication.
Keep existing CADPACK privacy/history/geometry tests as well.

The owner explicitly approved the three files and the patch is applied on the task branch. Formatting has been checked. No build
or execution is claimed. The owned unit-test benchmark now calls the hook.
Run the commands above and record actual raw results
at the same SHA for both candidate representations. Name the combined measurement
`cadpack_round_trip`: it includes writer, sync, read-back and validation time;
it is not a write-only timing. The copy/export gate remains unverified until the
real route executes and its proof succeeds.
