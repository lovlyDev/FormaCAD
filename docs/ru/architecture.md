# Архитектура
[Документация](index.md) · [English](../en/architecture.md)

| Слой | Назначение | Исходники |
| --- | --- | --- |
| React / Zustand | Рабочее место, UI и подтверждения | [App](../../apps/desktop/src/app/App.tsx), [store](../../apps/desktop/src/stores/workspace.ts) |
| Three.js | Отображение, камера, выбор, движение | [Viewer](../../apps/desktop/src/features/viewer/Viewer.tsx) |
| Tauri / Rust | Валидация, файлы, процессы, SQLite | [Бэкенд](../../apps/desktop/src-tauri/src/lib.rs) |
| CAD worker | Ограниченное исполнение CadQuery, STEP/GLB | [Worker](../../apps/desktop/src-tauri/scripts/model_program.py) |
| Обновления | Подписанные пакеты, настройки и бэкап | [UI](../../apps/desktop/src/components/Updates.tsx), [бэкенд](../../apps/desktop/src-tauri/src/updates.rs) |

[CAD-документ](../../apps/desktop/src-tauri/src/cad_document.rs) — версионированный JSON со стабильными ID операций, типами и ссылками на предыдущие операции. Поддерживаются rectangle, circle, extrude, translate и boolean. После проверки документ преобразуется во внутренний ограниченный Python. [Пример пластины с отверстием](../fixtures/plate-hole.cad.json) показывает зависимости.

Revision.program хранит JSON-документ либо совместимый Python-исходник. Версия схемы не зависит от версии приложения. Ревизии неизменяемы; восстановление создаёт новую. Ошибка построения не публикует успешную ревизию.

CLI получает только разрешённый контекст модели. Пути файлов и жизненный цикл процессов контролирует бэкенд. Геометрические и агентские процессы отделены от веб-интерфейса. Подробнее: [безопасность](../../SECURITY.ru.md) и [данные](data.md).
