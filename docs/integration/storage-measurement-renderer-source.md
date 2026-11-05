# Измерения выбранного тела, пары и инспектор: исходное открытие

[English](storage-measurement-renderer-source.en.md) · [Host API](../ru/storage-folder-host-api.md) · [Предыдущий preview-патч](storage-preview-renderer-source.md)

Статус: **подготовлен, не применён; требуется отдельное согласование**.
Issue #31 / PR #35, исполнитель wenlixxq, приёмка lovlyDev.
Основа `ecf40d55669701ec482fa3fc60a77a1f805df327` содержит применённый preview
и исправление строгого commit AI-review. Точный [патч](storage-measurement-renderer-source.patch)
не изменяет эти восемь файлов целиком: из пересекающихся путей App получает только
передачу binding в SelectionModelProperties.

## Поведение и данные

App передаёт binding открытия в инспектор и измерение выбранного тела. Запрос точных
свойств модели использует исходный FolderSourceRef; query cache включает opening,
source/task group и commit, поэтому поздний ответ или прежний cache не подставляются
при повторном открытии проекта с одинаковыми IDs. Явно отсутствующий/чужой binding
отключает запрос, сохраняя mesh estimate, без managed fallback.

ReferenceMeasurement и PairMeasurement захватывают исходный binding вместе с
геометрическим контекстом. Identity результата включает opening/source/task group,
revision/body/query, исходный CAD program и SHA/size основного STEP; пара также учитывает scene
token и выбранные topology references. Смена открытия сбрасывает выбор пары и
отклоняет поздний report, даже если project/revision/body IDs совпадают.

Metadata-only refresh того же открытия не сбрасывает совместимый report/выбор и
не переписывает capture уже работающего запроса. Session обновляет контекст только
следующего запроса на свежую commit-ссылку. Native API получает FolderSourceRef
самого capture; явная попытка подставить другой source/commit/group получает
MEASUREMENT_STALE до IPC. Прежние managed calls без binding сохраняются для
совместимых прямых callers; production UI передаёт binding явно.

Folder-состояния раскрытия reference/pair панелей остаются в RAM через source
preferences, без чтения/записи managed preferences или process-local tokens.
Старые callbacks capture/clear пары после смены контекста игнорируются и не стирают
выбор нового открытия. Нативный алгоритм и provenance report не меняются.

## Все 16 путей

Относительно `apps/desktop/src/`:

1. `features/reference-measurements/measurementCapture.ts`
2. `features/reference-measurements/measurementSession.ts`
3. `features/reference-measurements/useExactReferenceMeasurement.ts`
4. `features/reference-measurements/measurementApi.ts`
5. новый `features/project-sources/measurementSource.test.tsx`
6. `app/ModelInspector.tsx`
7. `features/reference-measurements/ExactReferenceMeasurement.tsx`
8. `features/reference-measurements/SelectionModelProperties.tsx`
9. `app/App.tsx`
10. новый `features/project-sources/inspectionSource.test.tsx`
11. `features/pair-measurements/pairCapture.ts`
12. `features/pair-measurements/pairApi.ts`
13. `features/pair-measurements/pairSession.ts`
14. `features/pair-measurements/usePairSlots.ts`
15. `features/pair-measurements/PairMeasurement.tsx`
16. новый `features/project-sources/pairSource.test.tsx`

## Проверки и пределы

В изолированной копии прошли **363 теста / 118 файлов**, production build, lint и i18n (1483 сообщения EN/RU). После применения нужны проверки actual head и CI; тесты наличием не подменяются.
Все остальные tracked frontend sources сравнены с основой и совпадают; новых
Rust/native/Cargo/backend IPC/SQL/версий или пользовательских строк нет.
15 новых renderer-тестов проверяют original routing, equal-ID reopen, cache isolation,
metadata continuation, следующую commit-ссылку, отказ foreign override и старые
callbacks. Это fixtures/mocks, не новый замер CAD или installed acceptance.

Папочные face-section, manual section, snap, history/import/export, writer/permission
UI и видимые folder controls этим патчем не подключаются. Этап 4 не завершён.
До согласования по §5 ТЗ общие исходники патча остаются неприменёнными.
