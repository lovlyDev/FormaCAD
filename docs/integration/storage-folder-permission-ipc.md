# Разрешения исходной папки через существующие IPC — точный патч

[English](storage-folder-permission-ipc.en.md) · [Host API](../ru/storage-folder-host-api.md) · [Сечения](storage-section-renderer-source.md)

Статус: **подготовлен; в рабочей ветке не применён; согласование владельца ещё не запрошено**.
Issue #31 / PR #35, исполнитель wenlixxq, согласование и приёмка lovlyDev.
Основа `3e945bca9cfb956b23ecdaafd4eb52e95bd0af37`. Точный
[патч](storage-folder-permission-ipc.patch) меняет 16 путей ниже. SHA256:
`02a11c3298fff552d2129188957dbacfd017a6f467e92194dcbd2ed5c709a8f0`.

## Контракт и хранение

Используются существующие `request_permission` и `resolve_permission`; дублирующего
IPC нет. Без folder-полей остаются прежние managed payload/ответ String. Managed
request/resolve вынесены в отдельный модуль; consume и его правила не меняются.

Folder request передаёт project ID, sourceId/expectedCommitId и типизированный intent:
Model с необязательным ReviewPlan, CAD Commands либо History. Неполная связка получает
отказ без managed fallback. Окно берётся из вызывающего WebviewWindow. Registry
захватывает один исходный FolderOperation; чистая подготовка получает его Project
и вызывает существующие валидаторы/command pipeline. Полный ApplyRequest, включая
ReviewPlan и revision, подготовленную Commands, сохраняется в RAM до решения и выполнения.
Построение использует тот же executor, без нового алгоритма геометрии.

Ответ folder request — `{id, requiresConfirmation}`. Этот флаг определяется только
существующей серверной confirmation policy. Режим none/override обрабатывает host;
флаг или настройка renderer не создают разрешение. Frontend overload сохраняет
исходный source/context и глубокую копию intent до await. Передать null вместо
folder-контекста или принять managed String как folder receipt нельзя.

При ручном решении host проверяет исходную связку до audit SQL. Разрешение становится
доступным после успешной записи решения и повторной проверки источника. Отказ
отзывает pending request даже при ошибке audit SQL. Audit rows/settings не восстанавливают
право после restart/close/reopen и не определяют источник; source/path/access tokens
в audit не записываются. Request ID остаётся диагностическим ID, а реальное разрешение
и команда существуют только в текущем RAM registry. Registry mutex не удерживается
через SQL await. Одноразовость, срок и обязательный OS writer lease остаются в backend.

Этот патч ещё не подключает writer-кнопки и существующие apply_program/apply_ir_commands/
undo/redo к folder receipt. Следующий согласуемый объём выполнит сохранённую команду
через эти существующие IPC и примет publication с новым commit. Видимая кнопка папки
остаётся скрытой до готовности полного сценария. Папочный editor/AI/command workflow
не объявляется готовым по одному permission API.

## Все 16 путей

1. `apps/desktop/src-tauri/src/permissions.rs`
2. новый `apps/desktop/src-tauri/src/permissions/managed_requests.rs`
3. новый `apps/desktop/src-tauri/src/permissions/folder.rs`
4. новый `apps/desktop/src-tauri/src/permissions/folder/host.rs`
5. новый `apps/desktop/src-tauri/src/permissions/folder/host/tests.rs`
6. новый `apps/desktop/src-tauri/src/permissions/folder/host/tests/worker.rs`
7. `apps/desktop/src-tauri/src/project_repository/window_sources/approval.rs`
8. `apps/desktop/src-tauri/src/project_repository/window_sources/approval/tests.rs`
9. `apps/desktop/src-tauri/src/project_repository/window_sources/supported.rs`
10. `apps/desktop/src-tauri/src/project_repository/window_sources/operation.rs`
11. `apps/desktop/src-tauri/src/project_repository/sqlite_folder/api/model.rs`
12. `apps/desktop/src/lib/api.ts`
13. новый `apps/desktop/src/features/project-sources/permissionContract.ts`
14. новый `apps/desktop/src/features/project-sources/permissionContract.test.ts`
15. `.github/workflows/storage-benchmark.yml`
16. `.github/workflows/storage-native-check.yml`

`open_for_test` в supported.rs существует только под cfg(test) и открывает настоящую
FCP1 fixture через реальный WindowSources, без диалога. Production-способ выбора
папки не меняется. Cargo/native/C++/worker protocol/Tauri capabilities/миграции/
версии/зависимости не меняются. Новых пользовательских текстов или стилей нет.

## Доказательства и изолированное CI-ревью

В отдельной копии с ровно этими исходниками прошли **407 frontend-тестов / 122 файла**,
production build, lint, i18n (1483 сообщения) и Rustfmt. Остальные отслеживаемые
frontend/Rust/workflow исходники совпадают с основой; `git apply --check` прошёл.
Десять новых transport-тестов проверяют legacy wire shape, исходный источник,
глубокий capture Commands/ReviewPlan, серверный флаг и отказ неверным receipts.

Первое изолированное CI-ревью 42309170343 подтвердило native-сборку, **567 library PASS / 20 ignored** и 11 smoke PASS. Все 12 новых SQL/FCP1 проверок и window-проверка чужого prepared request имеют явный PASS в терминальном логе. No-client/client strict Clippy и отдельный ignored worker-тест ещё не выполнены; эти условия остаются открытыми. SQL/FCP1 проверки покрывают
policy/overrides, malformed settings, zero managed project rows, wrong source/window/
project/commit, две физические папки с одним ID, audit failure/tampering, denial,
replay/reopen и SQL await без registry lock. Window-suite добавляет отдельный отказ
prepared request с чужим project/revision. Явный ignored native-тест проверяет Model
с ReviewPlan → отклонение неверного ожидаемого объёма без изменения папки → настоящую
CAD-команду 40→80 мм → Undo/Redo, включая automatic policy и инспекцию STEP.

Собственный [review workflow](../../.github/workflows/storage-permission-review.yml)
применяет SHA256-проверенный патч **только во временном CI checkout**, фиксирует
ровно 16 staged путей/хэши и запускает frontend, оба strict Clippy режима, SQL и
явный настоящий worker-тест. Ветка при этом содержит только review-артефакты,
а shared permission-код ждёт согласования по §5 ТЗ. Такой CI подтверждает предложенный
патч, а не его применение в runtime ветки. После согласования нужны проверки уже
применённого текущего SHA. Это не installed-app приёмка; локального Windows SDK нет.
