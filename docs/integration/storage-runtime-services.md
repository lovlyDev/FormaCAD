# Explicit modeling runtime services / Явные runtime services

Status: prepared, not applied. Base: approved C2 at `708877f`.
[Exact patch](storage-runtime-services.patch). One target:
`apps/desktop/src-tauri/src/modeling/repository_apply.rs`.

## RU

C2 отделяет runtime services от durable backend, но предоставляет только
конструктор `ApplyRuntime::from_state`. Для настоящего folder host без создания
ненужного AppState/глобальной SQLite требуется конструктор из существующих task map
и CAD gate. Патч добавляет только `ApplyRuntime::new(tasks, gate)` и делегирует ему
старый конструктор; managed поведение и области permissions, CAD execution,
публикации, IPC, native, Cargo и версий не меняются. Источник проекта, его SQL,
снимки и права не входят в этот конструктор. Owned folder adapter использует
настоящие task services; isolated integration tests не создают managed project rows.

Проверка применимости не доказывает компиляцию. После отдельного согласования
одного общего файла нужны форматирование, strict lint и folder/managed runtime
регрессии на актуальном head. Согласование требуется по §5 ТЗ.

## EN

C2 separates runtime services from durable storage but only provides
`ApplyRuntime::from_state`. The patch adds `new(tasks, gate)` and delegates the
existing constructor to it. A real folder host can use task services without
manufacturing AppState or opening global app SQLite. Project sources, snapshots,
authorization and durable SQL are absent from the constructor. Managed behavior,
permissions, CAD execution, publication, IPC, native, Cargo and versions stay
unchanged. Applicability alone is not compilation evidence; formatting, strict lint
and current managed/folder runtime checks remain required after separate approval.
