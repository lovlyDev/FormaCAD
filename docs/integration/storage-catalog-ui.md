# Managed catalog UI integration / Интеграция managed-каталога в UI

Base SHA: `e10c6438047e712a02f2ae53099dca0b251ba246`.
Patch: [storage-catalog-ui.patch](storage-catalog-ui.patch).
Prepared from copies only; production files were not edited or applied. Nine existing-file inputs verified against exact base blobs (LF normalized). Eight new modules/styles/tests. Total scope: 17 files.

## RU

Патч подключает approved CatalogReport к Dashboard: ready-карточки сохраняют текущие действия, unavailable показываются отдельно с ограниченными индексными подсказками, локализованным статусом, повтором чистого чтения и раскрываемыми техническими details. Нет фиктивного Project, открытия invalid ID, редактирования/экспорта/удаления непроверенной записи или выдуманной кнопки recovery. Строковый rowKey не преобразуется в число. Поиск недоступных строк использует имя/ID-подсказку.

Автоматический цикл построения и сохранения thumbnails с главной удаляется целиком. Список показывает уже сохранённый thumbnail либо стандартную иконку. Это устраняет именно фоновую запись Dashboard, не объявляет все действия приложения чистыми.

Новые useProjectCatalog/useProjectNavigation и cache helper вынесены в самостоятельные файлы. Открытие карточки заново читает target, не принимает старый Project как authority. Прямой route тоже читает один target, поэтому чужая unavailable-строка не блокирует здоровый проект. Общая generation для route/card запросов, исходный workspace pointer, unmount и busy guards отбрасывают поздний результат. Во время A→B чтения сцена A сохраняется, UI-настройки остаются в namespace A, редактирование модели блокируется и показан локализованный статус. При ошибке target-read сцена A сохраняется и URL восстанавливается на A; редиректа home/сброса сцены нет. Сбой каталога сам по себе не очищает workspace или Project[] cache.

Model-apply и revision events переходят с listProjects().find на target-read; существующие target/current-revision/generation/cache-before guards сохранены. Pin/rename перед записью получают текущий target. Повторный pin того же проекта во время собственного pending запроса игнорируется; это не distributed writer lock и не обещание атомарности против внешнего writer. Cache invalidation для catalog и прежнего Project[] cache централизована и не очищает действующий snapshot. Catalog остаётся optimistic-проекцией, не выдаёт writer lease.

Добавлены 5 navigation-регрессий (ошибка с восстановлением URL, поздний route, fresh card read, unmount, busy во время await), 2 параметризованных RU/EN card-теста с DOM-переключением light/dark и 1 cache-preservation тест. Прежние 3 event-regressions адаптированы к target-read, включая более поздние metadata того же head. Это присутствие тестов, **не PASS**: по разделению main выполняет formatter/build/tests. DOM theme tests не заменяют визуальную browser-приёмку обеих тем и языков.

`git apply --check` проходит; whitespace-only context нормализован; явная проверка patch text не нашла trailing whitespace. Патч НЕ применён. Cargo/native/worker/lib.rs/backend/миграции/версии не меняет. Явный recovery UI и standalone external-folder writer сюда не входят.

## Точные файлы / Exact paths

1. apps/desktop/src/app/App.tsx
2. apps/desktop/src/app/Dashboard.tsx
3. apps/desktop/src/features/model-apply/useModelApply.ts
4. apps/desktop/src/features/model-apply/useRevisionEvents.ts
5. apps/desktop/src/features/model-apply/useRevisionEvents.test.tsx
6. apps/desktop/src/features/project-catalog/UnavailableProjectCard.tsx (new)
7. apps/desktop/src/features/project-catalog/UnavailableProjectCard.css (new)
8. apps/desktop/src/features/project-catalog/UnavailableProjectCard.test.tsx (new)
9. apps/desktop/src/features/project-catalog/useProjectCatalog.ts (new)
10. apps/desktop/src/features/project-catalog/useProjectNavigation.ts (new)
11. apps/desktop/src/features/project-catalog/useProjectNavigation.test.tsx (new)
12. apps/desktop/src/features/project-catalog/cache.ts (new)
13. apps/desktop/src/features/project-catalog/cache.test.ts (new)
14. apps/desktop/src/i18n/en.json
15. apps/desktop/src/i18n/ru.json
16. docs/en/storage-catalog.md
17. docs/ru/storage-catalog.md

## EN

The patch connects the approved CatalogReport to Dashboard with visible ready and unavailable cards. Unavailable rows keep string row keys, bounded index hints, localized statuses, pure retry and separately expandable technical details. No fabricated Projects, invalid-ID opening, edit/export/delete actions or pretend recovery button are introduced.

The complete home thumbnail generation/save loop is removed. Saved thumbnails or the default icon remain. Modular catalog/navigation hooks use fresh target reads for cards and direct routes; unrelated damaged entries no longer block a healthy target. Shared request generations, original workspace comparison, unmount and busy guards reject late responses. During an A→B route read, A’s scene and UI namespace remain intact with model editing blocked; failure restores A’s URL while retaining its scene. Catalog failures do not clear authoritative cached Projects.

Model-apply/events switch to target reads while retaining existing guards. Pin/rename read current targets before existing writer calls. A repeated pin during its own pending request is ignored, without claiming an external-writer transaction guarantee. Both read-view caches are invalidated centrally and never cleared by that helper.

Eight new test cases cover navigation races/failures, localized unavailable cards and cache preservation; existing event tests are adapted. Tests/build/formatter were not run by this agent. Theme DOM toggles are not visual browser acceptance. git apply --check and explicit patch-whitespace inspection pass; production application has not occurred. No backend/Cargo/native/protocol/migration/version changes. Explicit recovery UI and an external-folder writer remain outside this patch.