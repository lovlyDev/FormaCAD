# Запись в исходную папку через существующие команды

[English](storage-folder-write-ipc.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **предложение для согласования в Issue #31, исходники не применены**.
Исполнитель wenlixxq, приёмка lovlyDev. База
`ad710626e0ee21b87ce6bf33170516f0b7d89faf`. Точный
[патч](storage-folder-write-ipc.patch), SHA256
`6926a08d0628f923483a28f986d8d71679aa1207a7325723c594ac35ea4328f3`.

## Сценарий

Существующие `apply_program`, `apply_ir_commands`, `undo_model` и
`redo_model` получают необязательный объект `folderWrite` с
`sourceId`, `expectedCommitId` и `approvalId`. Вызов также передаёт
`projectId`. Смешанный вызов с программой, CAD-командами, prompt,
ожидаемой ревизией или ReviewPlan отклоняется до погашения разрешения.
Команда берётся из уже одобренного запроса. Host проверяет окно, источник,
исходный коммит и вид операции под исходной блокировкой разрешения.

Совместимый managed-вызов сохраняет прежний JSON проекта и прежний маршрут
хранения. Folder-ответ привязан к `sourceId`, `projectId` и `baseCommitId`.
Он различает `noOp`, `committed`, `rejectedBefore` и `indeterminate`.
При неопределённости сохраняется ID попытки публикации. Frontend-помощник
проверяет ID по исходному разрешению и не повторяет вызов. Он передаёт
результат вызывающему коду для проверки текущего открытия перед заменой
сцены. Видимые кнопки, проверка неизвестного результата по commit ID и
чтение статуса Undo/Redo из папки остаются отдельными задачами.

Для исполнения используются существующие CAD worker и транзакционный
хост FCP1. Разрешение остаётся в RAM. IPC не принимает путь к рабочему
каталогу или worker и не создаёт управляемую строку проекта для папки.
Используются существующие коды ошибок и переводы.

## Файлы

| Область | Пути |
| --- | --- |
| Общие точки IPC | `apps/desktop/src-tauri/src/modeling.rs`, `apps/desktop/src-tauri/src/project_history/mod.rs` |
| Хранилище | `apps/desktop/src-tauri/src/project_repository/window_sources/{mod.rs,write_ipc.rs,write_ipc/tests.rs}` |
| Транспорт | `apps/desktop/src/features/project-sources/{writeApi.ts,writeApi.test.ts}` |
| Проверка | `.github/workflows/storage-benchmark.yml` |

Патч применим к указанной базе. В изолированной копии прошли rustfmt,
TypeScript, ESLint для новых модулей и 25 frontend-тестов (15 transport,
10 permission). Два новых Rust-теста проверяют форму запроса и
совместимость managed-ответа. Они и изменённый Tauri-маршрут требуют
компиляции и профильного CI после применения.
