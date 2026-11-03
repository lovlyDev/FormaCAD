# M2 snap retained snapshot / Исходный снимок M2 snap

Status: applied after lovlyDev explicitly approved the single target on 4 November
2026. Original patch base `bf9e854`; applicability rechecked on accepted-main
integration `2423200`. Current runtime verification remains required.
[Exact patch](storage-snap-snapshot.patch), one target:
`apps/desktop/src-tauri/src/model_snap/build.rs`.

## RU

Строгая компиляция run 41604274603 / job 125810473852 обнаружила E0609:
новый принятый M2 snap обращается к прежнему `CapturedMeasurement.project`.
Согласованный measurement adapter хранит исходный `CommittedSnapshot` в поле
`snapshot`. Патч меняет одну ссылку на `captured.snapshot.project`. Он сохраняет
исходный opening и существующие `capture::recheck` после очереди и worker await;
не захватывает новый сеанс для узаконивания старого ответа. Существующий source
seal и проверка результата worker остаются теми же. Полный маршрут прочитан.

Общий новый native consumer требует согласования по §5 ТЗ. Native implementation,
protocol, Cargo, permissions, IPC и версии не меняются. После разрешения нужны
strict Clippy и уже включённые в storage-native-check настоящие snap_host /
snap_worker / model_apply_transaction, без выключения тестов. Проверка
применимости не означает runtime PASS.

## EN

The exact-head strict compile found E0609: accepted M2 snap still accesses the
former `CapturedMeasurement.project`. The approved measurement adapter retains
the original committed snapshot under `snapshot`. This single-reference change
uses `captured.snapshot.project` for asset staging. Existing original-opening
rechecks after queue and worker awaits, source seals and report verification are
retained. No fresh session is captured to authorize an older result. The entire
snap host route was inspected. Native implementation, protocol, Cargo,
permissions, IPC and versions are unchanged. Section 5 requires approval for
this shared consumer. Strict Clippy and the already selected real-worker snap
and apply suites must pass afterward; applicability alone is not runtime proof.
