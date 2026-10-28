# План 2.0 → фактический статус на входе M0

[Индекс](README.md) · [Общий план](../../architecture-2.0.md) · [Статус продукта](../../ru/status.md)

Срез исходников: `4834e1020f755d4a158a105fc63965315d9c78c1` (1.5.1), 28 октября 2026. Проверка файлов и контрактов не равна успешной приёмке на устройстве. Старые личные handoff описывают более ранние версии и не используются как доказательство текущих тестов. Здесь нет утверждения о готовности архитектурной 2.0.

## Последовательность из §5 плана

| Шаг | Что есть в исходниках | Что остаётся / статус |
| --- | --- | --- |
| 1. Исходная точка и паритет | Синхронные версии 1.5.1; `docs/ru/modeling-parity.md`, существующие C++/Rust фикстуры и шесть шаблонов | Частично. Документирован неполный паритет сложных операций; не считать Python-совместимость нативной поддержкой. Новый инвентарь Blender не заменяет численные golden cases OCCT |
| 2. Версионные контракты | `cad_ir/mod.rs`: schema 2, Document/Body/Feature/Parameter/Operation/Command; строгая десериализация, `cad_ir/commands/`, `worker_protocol/`, схемы пакетов | Частично. Числовая schema 2 не означает полный охват новых операций; универсальные ссылки, новые команды и plugin-api 1.0 открыты. Формулы параметров не входят в 2.0 |
| 3. Нативный процесс | `bin/forma-cad-worker.rs`, `native/worker.rs`, `cad-core/`, раздельные native-worker-client/native-occt в Cargo.toml; очередь `cad_tasks` | Реализована основа, приёмка частичная. Нужны актуальные installed/offline/cancel/crash доказательства и все целевые ОС. Наличие worker не доказывает состав и работу установленного пакета |
| 4. Геометрический паритет | Типизированные примитивы, revolve многоугольного профиля, transforms, boolean, fillet/chamfer, STEP import | Открыт. Shell, sweep/loft, кривые, push/pull и параметрические массивы не закрыты. Нельзя убирать Python-путь, объявив все старые сценарии сохранёнными |
| 5. Проекты и транзакции | `project_storage_v2/`: generations, hashes, staging/recovery; `project_access/`, `project_history/`, `project_bundle/`; `project_repository/session.rs` содержит только Backend::LegacyManaged | Открыт. Самостоятельная папка как единый авторитет всех consumers, внешние location/session/epoch, NAS/USB и полный crash matrix не подтверждены. ReadSession явно не является OS lease или правом записи |
| 6. AI | `agents/custom.rs`, `cli_spec.rs`, `session/`, `review_plan/`, `selection/`, протокол команд; Codex/Claude/Custom представлены в коде | Частично. Проверить реальные авторизованные CLI, поток событий, ограничения повторов, привязку к ревизии, отмену и ясный план для пользователя; конфигурация адаптера не доказывает работающий внешний сервис |
| 7. Viewer | `features/viewer/`: R3F Canvas, OrbitControls, режимы и сечения; `reference-measurements/`, `pair-measurements/`, host `model_*` | Частично. Полного пути WebGPU и паритета fallback не найдено; универсальные pinned refs/recompute и все типы геометрии остаются. Требуются обе темы, RU/EN, реальный WebView и сохранение камеры |
| 8. Рабочее место | Dashboard, вкладки/панели, дерево, палитра, настройки, i18n, plugins UI | Частично. Произвольные области, рабочие пространства, все редакторы, доступность, реестр сочетаний и модальные инструменты из нового ТЗ открыты |
| 9. Упаковка и приёмка | CI/release workflows, Windows/OCCT build scripts, существующие релизные документы | Открыт. Ни старый EXE, ни наличие workflow не закрывают 10 сценариев §6, clean installed/offline на Windows/macOS arm64/x64/Linux и WebGPU/WebGL2. M0 не выпускает приложение |

## Приёмка §6: нужные доказательства

1. Чистая офлайн-установка: папка проекта → шаблон → правка → undo/redo → измерение → STEP/STL → повторное открытие.
2. Выбор грани/ребра многотельной модели → AI-план → подтверждение → проверка геометрии → отмена целого плана.
3. Ошибка последней команды: ни частичной геометрии, ни ревизии, ни новых pinned measurements; повтор от исходной ревизии.
4. STEP со структурой тел и mesh с честным типом: правка только допустимой геометрии и численный round-trip.
5. Неоднозначные ссылки, разорванные зависимости и неверная геометрия: структурированная ошибка, сохранённая последняя модель.
6. Убийство worker и приложения на стадиях commit: восстановление без повторного запуска AI/CAD.
7. Два процесса, NAS и отключение носителя: read-only, проверенная копия и прекращение записи при потере источника.
8. Полный/redacted cadpack: хеши, лимиты, traversal/bomb, отсутствие credentials, неактивные расширения и другая ОС.
9. WebGPU и WebGL2: выбор/мерки/сечение/X-Ray/Ghost, клавиатура, RU/EN, обе темы без сброса сцены.
10. Установка/обновление на всех целевых ОС без обязательного Python; отказ updater не блокирует CAD; бэкап миграции восстанавливается.

Ни один из этих полных сценариев не выполнялся в M0. Исторические результаты из status/release notes сохраняют свой исходный объём и не превращаются в свежий зелёный прогон.

## Границы этапов и владения

Текущая [Issue #17](https://github.com/lovlyDev/FormaCAD/issues/17) закреплена пользователем: ответственный lovlyDev, исполнитель Codex. Ветка `task/m0-blender-parity`; область — заметки и диагностика, без приложения, SQL, lockfile и общих версий. Чужие worktree, корневая dev/current и её незакоммиченные файлы сохраняются. Следующий этап — только после решения lovlyDev о слиянии M0 и закрепления его области. Задачи будущих этапов должны явно отличать планируемую работу от занятой.

Плановые задачи созданы без начала реализации и без неподтверждённого назначения исполнителя: [M1 #18](https://github.com/lovlyDev/FormaCAD/issues/18), [M2 #19](https://github.com/lovlyDev/FormaCAD/issues/19), [M3 #20](https://github.com/lovlyDev/FormaCAD/issues/20), [M4 #21](https://github.com/lovlyDev/FormaCAD/issues/21), [M5 #22](https://github.com/lovlyDev/FormaCAD/issues/22), [M6 #23](https://github.com/lovlyDev/FormaCAD/issues/23), [M7 #24](https://github.com/lovlyDev/FormaCAD/issues/24).

Проверяемые точки исходников: [операции CAD IR](../../../apps/desktop/src-tauri/src/cad_ir/mod.rs), [сеанс LegacyManaged](../../../apps/desktop/src-tauri/src/project_repository/session.rs), [маршрутизация AI CLI](../../../apps/desktop/src-tauri/src/agents/cli_spec.rs), [viewer](../../../apps/desktop/src/features/viewer/Viewer.tsx), [runtime плагинов](../../../apps/desktop/src-tauri/src/plugins/runtime/mod.rs).

Дополнение Blender не отменяет общий план: видимость — состояние вида, а геометрические изменения — Command API и транзакция. F9 должен согласоваться с append-only lineage и конфликтами ревизий; нельзя переписывать опубликованные ревизии ради интерфейсного сходства. Произвольные полигональные операции не добавляются в точный CAD. Публикация и установка требуют отдельного поручения.

English: this is a source-backed gap inventory at 1.5.1, not acceptance evidence. All nine execution slices remain partly open at the full architectural scope. In particular, LegacyManaged read identity is not standalone folder authority, registered AI adapters are not authenticated-provider acceptance, and the current R3F Canvas does not establish WebGPU/fallback parity. None of the ten complete installed cross-platform scenarios was executed during M0.
