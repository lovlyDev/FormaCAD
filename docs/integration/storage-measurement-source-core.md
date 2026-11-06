# Измерения через исходный источник — точный патч

[English](storage-measurement-source-core.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **применён после разрешения lovlyDev на все 10 файлов** 4 ноября 2026.
Основа `5899c64fe30bd5f83beb31caceaf7884acd56948`, Issue #31 / PR #35.
Ответственный — wenlixxq. Точный diff: [storage-measurement-source-core.patch](storage-measurement-source-core.patch).

## Контракт

Общий захват измерений хранит RepositoryRead исходного сеанса. Проверки исходной
папки, открытия и ревизии выполняются до и после CAD. Измерение тела, пары,
сечения грани и snap получают source-bound Rust entrypoints; существующие managed
entrypoints и тестовые hooks сохраняются. IPC и клиентское подключение этих
entrypoints в этот патч не входят.

Обновление имени или сообщений во время ожидания CAD допускается только после
повторного чтения через тот же RepositoryRead и сравнения исходной ревизии,
геометрических bindings, preview и группы открытия. Обновляется только проверенный
контекст чтения вложений. Close/reopen, другая папка с тем же ID или изменение
геометрии не принимаются как продолжение исходного запроса.

Новый модуль staging асинхронно читает проверенные STEP из исходного источника,
затем передаёт буферы существующему native stager без дополнительного клонирования.
Ограничения одного файла и суммарных 128 MB сохраняются. Native-код не меняется.

## Точные десять файлов

Все пути относительно `apps/desktop/src-tauri/src/`:

- `model_measurement/snapshot.rs`
- `model_measurement/capture.rs`
- `model_measurement/mod.rs`
- `model_measurement/build.rs`
- `model_measurement/build_tests.rs`
- `model_pair_measurement/build.rs`
- `model_face_section/build.rs`
- `model_snap/build.rs`
- новый `model_measurement/staging.rs`
- новый `model_measurement/folder_build_tests.rs`

Новые тесты используют настоящий импортированный STEP, отдельную FCP1-копию и
намеренно повреждённые managed-вложения с тем же project ID. Они проверяют
объём 8000 и размеры 40×20×10, metadata-only commit в очереди, сохранность сообщений,
отсутствие записи при чтении и отказ позднему результату после закрытия папки.
Существующие assertions не ослабляются; fixture-модуль получает только pub(super).

## Проверки и границы

На `64bde10e5fe5174e783d79e5d6f20ac0b3a32323` прошли оба строгих Clippy и
[comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41908269657),
[Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/41908279869) и
[native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/41908269630).
В скачанном логе job `126848656887` оба новых folder-теста явно имеют `ok`,
отдельный запуск дал ровно 2 PASS. Полный пакет: 537 library PASS / 18 ignored,
11 smoke PASS, 41 host consumer PASS и три отдельных roundtrip PASS.
Это проверка исходного reader/staging, не доказательство последующего IPC или UI.
У основы все три CI успешны: [Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/41817194409),
[comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187435),
[native](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187467).

Cargo, C++, worker protocol, permissions, SQL, версии и frontend не меняются.
Кнопки открытия папки и полный пользовательский маршрут этапа 4 ещё не завершены.
По §5 ТЗ общие measurement/snap файлы требуют отдельного согласования до применения.
