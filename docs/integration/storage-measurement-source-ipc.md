# Исходный источник измерений в существующем IPC

[English](storage-measurement-source-ipc.en.md) · [Патч основы](storage-measurement-source-core.md)

Статус: **применён после разрешения lovlyDev на все 8 файлов** 5 ноября 2026.
Основа `64bde10e5fe5174e783d79e5d6f20ac0b3a32323` прошла native-проверки.
Issue #31 / PR #35; wenlixxq — исполнитель, lovlyDev — согласование и приёмка.
Точный diff: [storage-measurement-source-ipc.patch](storage-measurement-source-ipc.patch).

## Контракт и восемь файлов

Четыре существующие команды получают необязательные sourceId и expectedCommitId.
Оба отсутствуют — прежний managed-маршрут; только одно поле — отказ до worker.
Оба присутствуют — WindowSources.capture_read проверяет настоящее окно, project ID
и commit, затем тот же source-bound CAD pipeline обрабатывает исходный сеанс.
Ошибка папки не приводит к managed fallback. taskScopeId не передаётся как authority.
Без native client запрос папки получает PROJECT_FOLDER_UNSUPPORTED.

Rust относительно `apps/desktop/src-tauri/src/`:

- `model_measurement/mod.rs`
- `model_pair_measurement/mod.rs`
- `model_face_section/mod.rs`
- `model_snap.rs`

TypeScript относительно `apps/desktop/src/`:

- `features/reference-measurements/measurementApi.ts`
- `features/pair-measurements/pairApi.ts`
- `features/viewer/sections/face-section/faceSectionApi.ts`
- `features/snapping/snapApi.ts`

API принимают необязательный FolderSourceRef и добавляют только два поля к
существующему запросу. Старые вызовы и проверки worker echo, source seal и
document SHA остаются. Browser-запрос папки отвергается перед invoke; действующий
перевод PROJECT_FOLDER_UNSUPPORTED уже есть. Новые пользовательские строки не добавляются.

## Проверки и ограничения

Применимость и staged rustfmt проверены. Полная проверка TypeScript frontend с
виртуальной подстановкой четырёх staged API-файлов прошла без изменения исходников.
После применения прошли все 294 frontend-теста в 104 файлах и production build.
Обнаруженная совместимость snap mock исправлена в том же разрешённом snap API:
isTauri проверяется только для folder-вызова, общий lib/api больше не импортируется.
Для `5cb38ef` прошли [Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/42042479753)
и [native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/42042470156): 537 library
PASS / 18 ignored, 11 smoke PASS, 41 host PASS, отдельные 2 measurement и три
roundtrip PASS. Оба строгих Clippy и packet-тесты прошли в
[comparison `3d8c767`](https://github.com/lovlyDev/FormaCAD/actions/runs/42044572105),
где исполняемый код совпадает с `5cb38ef`; разница — только preview-артефакт/документация.
Это подтверждает компиляцию и backend/native-маршрут, но не завершённый GUI-сценарий.
Узкий too_many_arguments allow
относится только к четырём Tauri-командам с совместимыми плоскими аргументами.

Workspace ещё должен передавать исходный FolderSourceRef и инвалидировать
запросы/выбор при смене или закрытии источника, даже если ID совпадают. Кнопки,
workspace state, история, ручное сечение и preview этим патчем не подключаются.
Cargo/native/C++/worker protocol/permissions/SQL/версии не меняются.
§5 ТЗ требует отдельного согласования общих IPC/frontend файлов до применения.
