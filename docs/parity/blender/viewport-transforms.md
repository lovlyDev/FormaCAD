# M2: независимые модели управления viewport

[RU](../../ru/viewport-transforms.md) · [EN](../../en/viewport-transforms.md)

Решение: адаптировать поведенческое описание G/R/S, ограничений осей, точек опоры, пространств, выделения и изоляции из согласованного задания. Реестр, числовая грамматика, математические функции и state machine написаны самостоятельно для FormaCAD. Исходники, декомпиляция, keymap/theme-файлы, иконки и другие ресурсы Blender не использованы.

Отличия и ограничения: длины нормализуются в мм; точная привязка требует ответа нашего OCCT worker, а не точки на mesh. Отражение отделено от положительного масштаба. Формулы с `=` пока отклоняются. Сессия привязана к конкретным проекту, ревизии и сцене; результат подтверждения не является автоматически выполненной транзакцией. Текущий этап — независимые модули до интеграции M1, а не полная приёмка M2.

Проверка: собственные геометрические примеры, поворот вокруг смещённой точки, русская раскладка, отмена, повторное подтверждение, точные единицы, выбранные/скрытые тела, region containment и отклонение устаревшего snap-ответа. Native affine primitive дополнительно проверяется независимыми измерениями и STEP. Проверки пользовательских сценариев и FPS должны быть выполнены после подключения адаптеров.

English: M2 uses independently authored behavior models for FormaCAD. No Blender source, decompilation, keymap/theme files or assets were used. Millimetre normalization, revision/scene captures and an authoritative OCCT snap boundary are FormaCAD decisions. These prepared modules do not yet constitute complete modal-tool or native transaction acceptance.

Дополнение F9: собственный host-маршрут заменяет последний логический слот Undo и сохраняет все физические ревизии неизменными. Текущий контракт ограничен матрицами последнего независимого пакета трансформаций тел. Реальный worker, Undo/Redo и portable archive проверены; интерфейс исходного инструмента ещё не подключён. Этот порядок хранения выбран для гарантий FormaCAD и не заявляется копией внутренней реализации Blender.

F9 update: the independently authored host route replaces the final logical Undo slot while retaining immutable physical revisions. Current support is limited to matrices in the latest independent body-transform batch. Real-worker persistence, Undo/Redo and portable archives are tested; modal UI integration remains pending. This is a FormaCAD storage decision, not a claim about Blender internals.
