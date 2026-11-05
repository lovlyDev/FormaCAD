# Источник workspace и защита поздних ответов — точный патч

[English](storage-workspace-source-state.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **подготовлен, не применён**. Основа `9604647389fa234fc87d57cd5a95b714a3b56d34`.
Issue #31 / PR #35; исполнитель wenlixxq, согласование и приёмка lovlyDev.
Diff: [storage-workspace-source-state.patch](storage-workspace-source-state.patch).

## Поведение и данные

Workspace хранит Project и отдельный process-local binding открытия. Managed metadata
сохраняет семейство запросов, явное открытие запускает новое; folder binding содержит
неизменяемую копию host FolderSourceRef. Локальный UUID защищает renderer-callback,
но не даёт native read/write authority. Host по-прежнему проверяет окно/source/commit.

Store получает явные managed/folder activation и compare-and-set публикацию нового
folder-снимка. Смена источника очищает topology/body selection; metadata того же
открытия сохраняет совместимый выбор. Поздний managed save/event/CAD result не
заменяет folder-view с теми же IDs. Managed persistence не вызывается для folder
read reference. Старый folder selector закрывается через существующий close_project_source.

Сцена учитывает открытие дополнительно к project/revision/asset seal, сразу скрывает
чужую геометрию и освобождает поздние объекты. Folder model loader читает именно
host-вложение даже при наличии inline-кандидата. После разбора модели выполняется
чистая проверка того же source/commit и file seal; при stale объект освобождается.
Готовая геометрия не перечитывается при metadata-only refresh. Pending IO получает
новую commit-ссылку и отклоняет старый объект.

Root-сохранение, основные managed authoring callbacks и CapturedApply получают
renderer opening guard. Поздняя ошибка освобождения managed-access не отображается
в другой source-view. Folder read reference не приобретает managed editing.
Статус managed-access также привязан к открытию: старый write-mode скрыт до
завершения приобретения доступа после повторного открытия того же ID.
Для отказа записи добавляется PROJECT_SOURCE_WRITE_NOT_AUTHORIZED в оба i18n.
Project-scoped черновики folder остаются в памяти; disabled persistence не читает
и не пишет managed preferences и не сохраняет source/epoch tokens. Старые setters
не меняют черновик нового открытия. Низкоуровневые managed интерфейсы сохраняются.

## Все 31 путей

Относительно `apps/desktop/src/`:

1. новый `features/project-sources/sourceBinding.ts`
2. новый `features/project-sources/sourceBinding.test.ts`
3. новый `features/project-sources/sourceApi.ts`
4. новый `features/project-sources/useWorkspaceSourceAccess.ts`
5. новый `features/project-sources/sourceAction.ts`
6. новый `features/project-sources/fileReadGuard.ts`
7. новый `features/project-sources/useSourcePreference.ts`
8. новый `stores/workspaceProject.ts`
9. новый `stores/projectDefaults.ts`
10. `stores/workspace.ts`
11. `lib/persistence.ts`
12. новый `lib/modelLoader.ts`
13. `lib/files.ts`
14. `features/model-apply/committedScene.ts`
15. `features/model-apply/committedSelectionScene.ts`
16. `features/model-apply/useCommittedScene.ts`
17. `features/project-catalog/useProjectNavigation.ts`
18. `features/model-apply/useRevisionEvents.ts`
19. `features/model-apply/capturedApply.ts`
20. `features/model-apply/useModelApply.ts`
21. `app/App.tsx`
22. `i18n/ru.json`
23. `i18n/en.json`
24. новый `stores/workspaceSource.test.ts`
25. новый `features/project-sources/sceneSource.test.tsx`
26. новый `features/project-sources/sourcePreference.test.tsx`
27. новый `features/project-sources/managedRequests.test.tsx`
28. новый `features/project-sources/fileSource.test.ts`
29. новый `features/project-sources/sourceAccess.test.tsx`
30. `features/project-access/useProjectAccess.ts`
31. новый `features/project-sources/managedAccessScope.test.tsx`

Model loading и project-state transitions вынесены из смешанных files/workspace
модулей; re-exports сохраняют существующие imports. Rust, native, Cargo, worker
protocol, native permissions, SQL, версии и backend IPC-контракты не меняются.

## Проверки и незавершённый объём

В изолированном полном снимке основы с этими 31 файлами прошли 332 тестов / 112
файлов, production build, check:i18n (1483 EN/RU сообщения) и check:docs (190 документов
основы). В их числе 38 новых тестов состояния/рендерера: same-ID switch/reopen,
metadata refresh, late save/event/CAD adoption, source-scoped geometry, отказ после
decoder await, очистка объекта, отдельные preferences и read-only access. Проверена
применимость точного патча к рабочей ветке. Общие working sources не изменены.

Эти frontend-тесты используют renderer fixtures/mocks и не подменяют настоящие CAD
проверки. Native preview-основа `4b96158` прошла все три CI. После применения патча
нужны повторные проверки actual head, включая браузерные сценарии по мере wiring.

Это основа store/scene и основных callbacks. Патч **не добавляет кнопку папки**,
не подключает все remaining geometry/history/import/export consumers и не реализует
folder writer/permissions UI. Read-only folder binding — не конечный результат
этапа 4: до завершения read/write маршрута источник не включается в видимый flow.
Отдельный manual-section патч остаётся несогласованным и сюда не включён.
§5 ТЗ требует согласования этих shared store/App/frontend/i18n файлов до применения.
