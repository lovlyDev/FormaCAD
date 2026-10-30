# Сеанс точных измерений / Exact measurement session

## Русский

lovlyDev разрешил и применён точный первоначальный [патч](storage-measurement-session.patch)
для шести файлов: `model_measurement/{snapshot,capture,build,build_tests}.rs`,
`model_pair_measurement/build.rs`, `model_face_section/build.rs`. Исходная база —
`main` @ `4834e1020f755d4a158a105fc63965315d9c78c1`; приёмка остаётся у lovlyDev.
Патч — исторический артефакт согласования, а не текущий полный diff.

`CapturedMeasurement` удерживает исходный `CommittedSnapshot` через ожидание
очереди и работу worker. Старый ответ не получает новый сеанс при recapture.
Сохраняются expected revision/body, document/source hash и размер STEP, preview
seal, imported asset seals, request/report echo и cancellation. Native protocol,
C++/Cargo, IPC и пользовательские строки не меняются.

Коррекция поведения 31 октября: при первоначальном capture остаётся полная
проверка согласованного сохранённого snapshot. При повторной проверке геометрии
исходный `recheck_lifecycle` вызывается до и после нового capture. Он проверяет
исходные opening/epoch/location и неизменность захваченных данных в памяти, но
не требует совпадения всего текущего payload. Затем сравниваются revision,
binding и preview. Сохранение сообщения при неизменной геометрии должно оставлять
запрос действительным; close/reopen, подмена источника и изменение геометрических
входов по-прежнему отклоняют результат. Нельзя заменять этим lifecycle-only
методом полную проверку сохранённого состояния для copy/export/history.

Причина коррекции: первоначальная полная legacy::recheck в финальном
measurement recheck отклоняла и metadata-only save. Это противоречило трём
существующим `queued_metadata_only_save_preserves_messages_and_does_not_invalidate_inputs`
в host integration suites. Прежний native прогон исполнял только lib/native_worker,
а compile all-targets не означал исполнения этих трёх suites.

[Native job 123428058352](https://github.com/lovlyDev/FormaCAD/actions/runs/40868152101/job/123428058352)
на `147a0dd` подтвердил исходную close/reopen регрессию с настоящим worker.
Это не результат исправленного head. Новый workflow `Storage native consumers`
использует существующий build-native-unix.sh в TEST_ONLY режиме и дополнительно
исполняет `reference_measurements_host`, `pair_measurements_host` и
`face_sections_host`. Он не собирает установщик и не меняет скрипт владельца.
Результат коррекции ожидает исполнения. Гарантия остаётся optimistic; реальный
NAS/USB и установленное приложение этим не подтверждаются.

## English

lovlyDev approved the initial [patch](storage-measurement-session.patch), applied
to six files: `model_measurement/{snapshot,capture,build,build_tests}.rs`,
`model_pair_measurement/build.rs` and `model_face_section/build.rs`. Its original
base was `main` @ `4834e1020f755d4a158a105fc63965315d9c78c1`; lovlyDev retains
acceptance. The patch is the historical approval artifact, not the current full diff.

`CapturedMeasurement` retains its original `CommittedSnapshot` across queue waits
and worker execution. Recapture cannot grant a new session to an old result.
Expected revision/body, document/source hash and STEP size, preview seal,
imported asset seals, request/report echo and cancellation checks remain.
No native protocol, C++/Cargo, IPC or user strings change.

October 31 behavior correction: initial capture retains complete consistent
saved-state verification. Geometry recheck now calls the original
`recheck_lifecycle` before and after recapture. It verifies original
opening/epoch/location and captured in-memory data integrity, without requiring
all current payload bytes to match. Revision, binding and preview are then
compared. A message-only save with unchanged geometry must preserve the request;
close/reopen, source replacement and geometry input changes still reject it.
Lifecycle-only checks must not replace saved-state checks for copy/export/history.

The initial final legacy::recheck also rejected metadata-only saves, contrary to
three existing host integration regressions named
`queued_metadata_only_save_preserves_messages_and_does_not_invalidate_inputs`.
Earlier native execution covered lib/native_worker only; compiling all targets
did not execute these three suites.

[Native job 123428058352](https://github.com/lovlyDev/FormaCAD/actions/runs/40868152101/job/123428058352)
on `147a0dd` proved the original real-worker close/reopen regression. It does not
verify the corrected head. The new `Storage native consumers` workflow uses the
existing build-native-unix.sh TEST_ONLY route and additionally executes
`reference_measurements_host`, `pair_measurements_host` and `face_sections_host`.
It neither builds an installer nor modifies the owner's script. Correction
execution is pending. Guarantees remain optimistic; real NAS/USB and an installed
application are not established by these checks.