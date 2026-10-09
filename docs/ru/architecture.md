# Архитектура
[Документация](index.md) · [English](../en/architecture.md)

Подробные сценарии и границы функций: [карта кода](internals.md), [моделирование](modeling.md), [AI](ai.md), [проекты](projects.md), [просмотр](viewer-and-files.md).

| Слой | Назначение | Исходники |
| --- | --- | --- |
| React / Zustand | Рабочее место, UI и подтверждения | [App](../../apps/desktop/src/app/App.tsx), [store](../../apps/desktop/src/stores/workspace.ts) |
| Three.js | Отображение, камера, выбор, движение | [Viewer](../../apps/desktop/src/features/viewer/Viewer.tsx) |
| Tauri / Rust | Валидация, файлы, процессы, SQLite | [Бэкенд](../../apps/desktop/src-tauri/src/lib.rs) |
| Нативный CAD worker | Изолированное исполнение CAD IR v2 в C++/OpenCascade; точный STEP и GLB-предпросмотр | [Worker](../../apps/desktop/src-tauri/src/bin/forma-cad-worker.rs), [ядро](../../cad-core/CMakeLists.txt) |
| Worker совместимости | Ограниченное исполнение CadQuery для старых моделей | [Python worker](../../apps/desktop/src-tauri/scripts/model_program.py) |
| Обновления | Подписанные пакеты, настройки и бэкап | [UI](../../apps/desktop/src/components/Updates.tsx), [бэкенд](../../apps/desktop/src-tauri/src/updates.rs) |

[Документ CAD IR v2](../../apps/desktop/src-tauri/src/cad_ir/mod.rs) содержит стабильные ID параметров, операций и тел, типизированные операции и ссылки на предыдущие шаги. Перед построением проходит [валидацию](../../apps/desktop/src-tauri/src/cad_ir/validation.rs). [Зависимости операций](../../apps/desktop/src-tauri/src/cad_ir/dependencies.rs) позволяют подавлять шаги и откатывать результат каждого тела; [сравнение ревизий](../../apps/desktop/src-tauri/src/cad_ir/diff.rs) показывает структурные изменения. Ограниченный [solver 2D-эскизов](../../apps/desktop/src-tauri/src/cad_ir/sketch.rs) рассчитывает локальные связи точек, затем [нативный исполнитель](../../apps/desktop/src-tauri/src/native/document.rs) строит точную BREP-геометрию. Для выбранных рёбер прямоугольных тел есть ограниченный [семантический селектор](../../cad-core/src/box_edge_selector.cpp): при пересборке ссылка разрешается заново. Старые документы v1 используют [адаптер совместимости](../../apps/desktop/src-tauri/src/cad_document.rs); [пример пластины с отверстием](../fixtures/plate-hole.cad.json) относится к этой схеме. Поддерживаемые связи и ограничения описаны в [руководстве по эскизам](sketches.md).

Revision.program хранит JSON-документ либо совместимый Python-исходник. Версия схемы не зависит от версии приложения. Ревизии неизменяемы; восстановление создаёт новую. Ошибка построения не публикует успешную ревизию.

CLI получает только разрешённый контекст модели. [Нативный планировщик](../../apps/desktop/src-tauri/src/agents/repair.rs) проверяет геометрию кандидата в изолированном worker и допускает максимум две попытки исправления с исходными требованиями по структурированной ошибке ядра. Пути файлов и жизненный цикл процессов контролирует бэкенд. Геометрические и агентские процессы отделены от веб-интерфейса. Подробнее: [безопасность](../../SECURITY.ru.md) и [данные](data.md).
