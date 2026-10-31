# Storage catalog IPC contract patch / Патч IPC-контракта каталога

Base SHA: `12acee63eb54b353eab47979853acb501f674549`.
Patch: [storage-catalog-contract.patch](storage-catalog-contract.patch).
Prepared from exact `git show BASE:path` copies; never applied to production files.

## RU

Это первый согласуемый патч backend+wire contract, **не завершённый Dashboard**. Изменяет 12 файлов, 141 вставка/21 удаление. `list_projects` теперь возвращает существующий чистый CatalogReport вместо reconcile каждой строки. Host задаёт продуктовые пределы 1000 строк/128 МиБ ready JSON (не peak-memory bound). Новый `read_project_snapshot` читает один настоящий сохранённый Project через managed capture/recheck без recovery/lease. Существующего безопасного get IPC нет: `projects::get` вызывает reconcile и здесь не используется. `projects::get`, writer, recovery, native/Cargo/versions не меняются.

Frontend получает отдельные catalog/target adapters. Старый `listProjects(): Project[]` строго отклоняет частичный report с локализованной ошибкой, сохраняя весь report в `PartialCatalogError`; не фильтрует unavailable и не создаёт фиктивный Project. Browser localStorage route остаётся прежним. Технические details не попадают в основной локализованный текст. Добавлены RU/EN строка и обновления тематической документации.

Добавлены SQL-регрессия смешанного списка/target-read с неизменными источниками/pending journal/closed access и два frontend-теста совместимости/rowKey выше 2^53. **Тесты и formatter не запускались**, по разделению main — единственный runner. `git apply --check` прошёл на текущем рабочем дереве 12acee6; это доказывает применимость, не компиляцию.

Оставшаяся интеграция отдельным патчем: Dashboard карточки всех unavailable; target-read для App/model-apply/events; отсутствие навигации/очистки workspace при ошибке списка; убрать автоматическую запись thumbnails с главной; единая invalidation между authoritative cache и catalog. Пока новый catalog API не подключён к Dashboard, старый UI показывает общую ошибку при частичном каталоге. Нет фиктивной recovery-кнопки или обещания уже доступного сценария. Если промежуточный breaking IPC contract нежелателен, объединить этот патч с следующим frontend-патчем до merge, а не выдавать его за готовую пользовательскую функцию.

Точные пути:
- apps/desktop/src-tauri/src/projects.rs
- apps/desktop/src-tauri/src/projects/catalog.rs (new)
- apps/desktop/src-tauri/src/projects/catalog/tests.rs (new)
- apps/desktop/src-tauri/src/lib.rs
- apps/desktop/src/lib/api.ts
- apps/desktop/src/features/project-catalog/api.ts (new)
- apps/desktop/src/features/project-catalog/contract.ts (new)
- apps/desktop/src/features/project-catalog/contract.test.ts (new)
- apps/desktop/src/i18n/ru.json
- apps/desktop/src/i18n/en.json
- docs/ru/storage-catalog.md
- docs/en/storage-catalog.md

## EN

This is the first backend/wire-contract patch, not completed Dashboard integration. It reuses list_projects for the pure CatalogReport, with explicit host response budgets (1000 rows/128 MiB ready JSON), and adds one pure target snapshot command because existing projects::get performs reconciliation. Reads retain repository capture/recheck and do not recover, obtain a writer lease or mutate sources. Cargo/native/version/recovery/writer code is unchanged.

Frontend adapters expose the report and target reads. Existing Project[] callers reject any unavailable entry using a localized PartialCatalogError retaining the entire report; they neither filter it nor fabricate Projects. Browser storage keeps its existing route. New tests cover mixed catalog/target purity and strict frontend compatibility with large row keys. Tests/formatter were not run; main owns runners. Only git apply --check has passed.

The next patch still needs unavailable cards, App/model-apply/event target reads, workspace preservation on catalog failures, centralized cache invalidation, and removal of automatic home thumbnail writes. No recovery button is invented. Keep the PR draft or combine with the coherent frontend integration before merge if an intermediate IPC return-type change is undesirable.

Browser target-read uses the existing localized `Project was not found` key (both catalogs verified). Artifact-only publication; no production application.

Applied after explicit lovlyDev approval of all twelve paths. Main ran rustfmt, frontend build/lint, 11 targeted tests, docs/i18n checks successfully. Added catalog limit/unavailable translations and two locale regressions within approved paths. Rust runtime validation remains CI-only; Dashboard integration is still separate.
