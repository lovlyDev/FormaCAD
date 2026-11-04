# Вложения папки в существующем IPC — точный патч

[English](storage-folder-attachments.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **подготовлен, не применён**. Основа —
`1c15f96ba607d2745c26889c889696b27a15ee42`, Issue #31 / PR #35.
Ответственный — wenlixxq, согласование общей области и приёмка — lovlyDev.

## Контракт и шесть путей

- `apps/desktop/src-tauri/src/artifacts.rs`: существующий `read_project_file`
  сохраняет managed-вызов `{projectId, name}`; folder-вызов дополнительно требует
  `{sourceId, expectedCommitId}` вместе и вызывает `WindowSources::read_asset`
  с настоящим вызывающим окном Tauri. Неполная пара получает отказ, не fallback.
- `apps/desktop/src/lib/api.ts`: `readProjectFile` принимает необязательный
  `FolderSourceRef`; без него сохраняет прежние IPC-аргументы, с ним передаёт
  ключ и commit. Browser-ветка с folder ref отказывает до invoke.
- `apps/desktop/src/features/project-catalog/api.ts`: такой же необязательный
  ref для уже подключённого `readProjectSnapshot`; browser не подменяет folder
  ref поиском managed-проекта с совпавшим ID.
- `apps/desktop/src/features/project-sources/contract.ts`: общий тип
  `{readonly sourceId, readonly commitId}`, непостоянный ключ окна, не writer grant.
- `apps/desktop/src/i18n/ru.json` и `en.json`: переводы `PROJECT_SOURCE_LIMIT`
  и `PROJECT_FOLDER_UNSUPPORTED`, включая browser-отказ через i18n.

Данные возвращаются в прежнем формате data URL. Размер, checksum и членство файла
берутся из сохранённого снимка, проверяются существующим bounded CAS reader;
исходный view повторно проверяется после кодирования. Произвольный путь или
описатель UI не становятся authority, глобальная SQLite не подменяет источник.
Общий IPC не дублируется, legacy Rust helper остаётся совместимым.

## Проверки и ограничения

Точный diff: [storage-folder-attachments.patch](storage-folder-attachments.patch).
Проверены применимость `git apply --check` и rustfmt подготовленного Rust-файла.
Общий патч не скомпилирован и не применён. Тесты host reader в собственной области
отправлены отдельно в `1c15f96`: две настоящие папки с одинаковыми ID/именами и
разными байтами, исходный SQL без изменений при чтении, чужой commit, отсутствующее
имя, обход пути, отказ старому commit после публикации и закрытому источнику.
Основа прошла [Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41807057154):
98 folder-тестов и два window/asset-теста, оба строгих Clippy-режима и оба
измерительных шага. Это не подтверждает компиляцию ещё не применённого общего патча.

После согласования необходимы strict Clippy, window/asset packet, build,
check:i18n и существующие frontend-проверки на точном применённом SHA.
Согласование требуется §5 ТЗ для `artifacts`, frontend API и i18n. Кнопка,
workspace source state, geometry/model IPC и recent index этим патчем ещё не
подключаются; пользовательский сценарий папки не объявляется завершённым.
Cargo/native/protocol/permissions/SQL-схемы/версии не меняются.
