# Карта кода и маршруты данных

[Документация](index.md) · [English](../en/internals.md) · [Архитектура](architecture.md) · [Моделирование](modeling.md)

Этот документ помогает найти место изменения, а не заменяет пользовательские руководства. Исходники состоят из npm workspace настольного React-приложения, Tauri/Rust-бэкенда, C++ CAD-ядра, Python-маршрута совместимости, скриптов проверок и workflow сборки. [package.json](../../package.json) содержит верхнеуровневые команды, [Tauri entry](../../apps/desktop/src-tauri/src/lib.rs) регистрирует плагины и команды. Номер версии приложения отдельно синхронизируется между npm, Cargo, Tauri и lockfile.

## Где менять функцию

| Область | Основные файлы | Ответственность |
| --- | --- | --- |
| Рабочее место | [App.tsx](../../apps/desktop/src/app/App.tsx), [workspace.ts](../../apps/desktop/src/stores/workspace.ts), [persistence.ts](../../apps/desktop/src/lib/persistence.ts) | Навигация, состояние UI, восстановление на старте. |
| Проекты | [Dashboard.tsx](../../apps/desktop/src/app/Dashboard.tsx), [projects.rs](../../apps/desktop/src-tauri/src/projects.rs), [cadpack.rs](../../apps/desktop/src-tauri/src/cadpack.rs) | Карточки, жизненный цикл проекта, перенос пакета. |
| CAD IR | [mod.rs](../../apps/desktop/src-tauri/src/cad_ir/mod.rs), [commands.rs](../../apps/desktop/src-tauri/src/cad_ir/commands.rs), [validation.rs](../../apps/desktop/src-tauri/src/cad_ir/validation.rs), [dependencies.rs](../../apps/desktop/src-tauri/src/cad_ir/dependencies.rs) | Формат, редактирование, ссылки и инварианты. |
| Эскизы | [sketch.rs](../../apps/desktop/src-tauri/src/cad_ir/sketch.rs), [Sketch2dEditor.tsx](../../apps/desktop/src/components/Sketch2dEditor.tsx) | Решение связей и редактор 2D-профиля. |
| Нативная геометрия | [worker.rs](../../apps/desktop/src-tauri/src/native/worker.rs), [document.rs](../../apps/desktop/src-tauri/src/native/document.rs), [C++ core](../../cad-core/src/) | Межпроцессный запуск, B-Rep построение, STEP/GLB. |
| Старые модели | [cad_document.rs](../../apps/desktop/src-tauri/src/cad_document.rs), [legacy.rs](../../apps/desktop/src-tauri/src/modeling/legacy.rs), [model_program.py](../../apps/desktop/src-tauri/scripts/model_program.py) | CadQuery-совместимость и ограниченное исполнение. |
| AI и процессы | [agents](../../apps/desktop/src-tauri/src/agents/), [processes](../../apps/desktop/src-tauri/src/processes/), [permissions.rs](../../apps/desktop/src-tauri/src/permissions.rs) | CLI, контекст, прогресс, отмена, разрешения. |
| Просмотр | [Viewer.tsx](../../apps/desktop/src/features/viewer/Viewer.tsx), [CameraControl.tsx](../../apps/desktop/src/features/viewer/CameraControl.tsx), [model.ts](../../apps/desktop/src/lib/model.ts) | Three.js, камера, выбор, сетки. |
| Файлы | [files.ts](../../apps/desktop/src/lib/files.ts), [files.rs](../../apps/desktop/src-tauri/src/files.rs), [artifacts.rs](../../apps/desktop/src-tauri/src/artifacts.rs), [conversion.rs](../../apps/desktop/src-tauri/src/conversion.rs) | Импорт, экспорт, вложения и конверсия. |
| Данные и обновления | [storage.rs](../../apps/desktop/src-tauri/src/storage.rs), [updates.rs](../../apps/desktop/src-tauri/src/updates.rs), [Updates.tsx](../../apps/desktop/src/components/Updates.tsx) | SQLite, миграции, проверка и установка версии. |

## Маршрут правки модели

Панель параметров или эскиза формирует команду → [Tauri API](../../apps/desktop/src/lib/api.ts) вызывает `apply_ir_commands` → [modeling.rs](../../apps/desktop/src-tauri/src/modeling.rs) применяет пакет к ожидаемой ревизии → CAD IR проверяет структуру и зависимости → [native worker](../../apps/desktop/src-tauri/src/bin/forma-cad-worker.rs) строит и проверяет результат → успешные документ и артефакты сохраняются в новой ревизии → React получает обновлённую модель. При ошибке текущая успешная ревизия не заменяется. Изменения AI проходят дополнительный этап плана и его проверки; подробности в [AI-руководстве](ai.md).

SQLite — источник метаданных проектов и настроек; файловые артефакты лежат под `projects/`. Оба нужны для полной резервной копии. Для схемы добавляйте **новую** SQL-миграцию в [migrations](../../apps/desktop/src-tauri/migrations/), не редактируйте применённую; окончания строк должны быть LF. Это проверяет `npm run check:migrations`. Подробнее в [данных](data.md).

## Маршрут сборки и документации

Новые пользовательские тексты сразу добавляются в [ru.json](../../apps/desktop/src/i18n/ru.json) и [en.json](../../apps/desktop/src/i18n/en.json). После каждой функции обновляются оба тематических руководства и ссылки; [порядок](documentation.md) обязателен. Локальные проверки задаются [package.json](../../package.json), [CI workflow](../../.github/workflows/ci.yml) и [руководством разработки](development.md). [Release workflow](../../.github/workflows/release.yml) запускается только новым тегом; публикация объединённой 1.2.6 разрешена после итоговых проверок. [Статус](status.md) отделяет проверенные локальные сборки от планируемых платформенных выпусков.

## Модули вспомогательной геометрии

[Редактор линий](../../apps/desktop/src/components/sketch/SketchConstructionEditor.tsx) отделён от редактора плоскости и точек. [Изменения черновика](../../apps/desktop/src/lib/sketchConstruction.ts) сохраняют общие точки при удалении. Rust-модуль эскиза разделён на [структуру](../../apps/desktop/src-tauri/src/cad_ir/sketch/structure.rs), [численный solver](../../apps/desktop/src-tauri/src/cad_ir/sketch/solver.rs) и [контур](../../apps/desktop/src-tauri/src/cad_ir/sketch/outline.rs). [Руководство](sketch-construction.md) описывает хранение, маршрут построения и ограничения.

## Sketcher 1.2.6

Редактор разделён на инструменты контура, точек, связей, canvas и диагностики в `src/components/sketch/`; параметры — `CadParameterEditor`. Чистые изменения документа и очистка ссылок находятся в `src/lib/sketch*` и `cadParameters`. Read-only `analyze_sketch` использует shared system/solver/profile, возвращая ранг и невязки; kernel и SQLite в анализе не участвуют. Build отправляет нормализованные контуры в `profile_prism.cpp` через CXX. См. [профили](sketch-profiles.md), [связи](sketch-constraints.md), [параметры](cad-parameters.md), [диагностику](sketch-diagnostics.md).
