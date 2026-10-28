# Области и рабочие пространства: реализация M1

[Руководство RU](../../ru/workspace-areas.md) · [Guide EN](../../en/workspace-areas.md)

Назначение: адаптация описанного и наблюдаемого поведения областей Blender к существующей FormaCAD, без переноса исходников или оформления Blender. Спецификация раскладки составлена своими словами в согласованном задании M1. Код Blender, декомпиляция, его файлы раскладок, тем, иконки и ресурсы при реализации не использовались.

Поведение: область имеет редактор; разделение создаёт соседнюю область, объединение отдаёт место соседней ветви; разворот временно скрывает остальные. Рабочее пространство сохраняет расположение редакторов. В FormaCAD это собственное бинарное дерево с проверяемой JSON-версией и плоским списком живых редакторов. Один экземпляр редактора можно переставить или обменять местами с другим. Это явное ограничение текущей реализации, а не утверждение о поведении Blender.

Решение: адаптировать для точного CAD существующие модель, параметры, свойства, AI, файлы, историю, очередь и события. Не добавлять пустые редакторы будущих узлов, макросов, сборок и чертежей. Выделение не уводит пользователя из AI. Редакторы геометрии сохраняют Command API и подтверждение.

Модули: `features/workspace-areas/`, `features/workspaces/`, `stores/layout/`, отдельные `outliner/`, `properties/`, `assistant/`, `project-files/`, `project-history/`, `activity-log/`, `main-menu/`, `status-bar/`, `settings/`, `viewport-sidebar/`.

Проверка: численные инварианты дерева/геометрии в unit-тестах и браузерные сценарии на настоящем WebGL2. Canvas и его контекст должны оставаться теми же после перестройки, темы и разворота. Ограничения: Chromium вместо installed WebView, профильное хранение UI вместо переносимого проекта, отсутствие замеров 50 FPS модального инструмента (такого инструмента в M1 нет). Полная приёмка 2.0 остаётся отдельной задачей.

English: M1 independently implements an area tree and stable editor hosts for FormaCAD. No Blender code, decompilation, keymap/theme files or assets were used. Editor swapping, a single instance per editor and application-profile persistence are explicit FormaCAD limits. Browser tests prove canvas/context identity within Chromium, not complete installed or cross-platform acceptance.
