# Сечения отображаемого исходного открытия — frontend

[English](storage-section-renderer-source.en.md) · [Host API](../ru/storage-folder-host-api.md) · [Ручной STEP backend](storage-manual-section-source.md)

Статус: **применён после явного согласования всех девяти файлов владельцем**.
Issue #31 / PR #35, исполнитель wenlixxq, согласование и приёмка lovlyDev.
Основа `bc9878a2c76e9df29372ede720f4476497259b94` плюс уже согласованный и
применённый шестифайловый manual-section патч. Точный [diff](storage-section-renderer-source.patch)
не включает Rust, Cargo, native, backend IPC, SQL, permissions или версии.

## Пользовательское поведение и маршрут

Существующая панель сечений передаёт исходное открытие как ручному STEP-сечению,
так и сечению выбранной грани. В backend API используется исходный FolderSourceRef;
чужой override для face capture отклоняется. Ручной запрос содержит исходный commit
и после metadata refresh запускается заново с новым commit. Старый запрос не
перенацеливается. Face-запросы сохраняют совместимый геометрический результат при
metadata refresh, а следующий запрос получает новую исходную ссылку.

`sceneRead` разделяет проверку того же открытия и текущей геометрии. Это позволяет
оставить локальную GPU-плоскость на удерживаемом объекте того же открытия во время
замены геометрии. Новые точные кривые ожидают фактически отображаемую актуальную
сцену. Объект из другой папки, даже с теми же project/revision ID, не заимствует
новый источник. Folder-объект без WeakMap-привязки недоступен для точного запроса.
Process-local ссылки не записываются в GLB/JSON.

Manual result/error принимает только исходный актуальный snapshot. Переключение
источника скрывает прежний ответ; общий порядок одного незавершённого native вызова
между manual и face режимами сохраняется. Старый callback явного face Calculate
не запускает команду в новом открытии. Folder-настройки панели хранятся в RAM,
сбрасываются при новом открытии, не читают и не перезаписывают managed preferences.
Metadata refresh того же открытия сохраняет эти настройки.

## Все девять путей

Относительно `apps/desktop/src/`:

1. `features/viewer/sections/face-section/faceSectionCapture.ts`
2. `features/viewer/sections/face-section/faceSectionApi.ts`
3. `features/viewer/sections/face-section/useFaceSection.ts`
4. новый `features/project-sources/faceSectionSource.test.tsx`
5. `features/project-sources/sceneRead.ts`
6. новый `features/viewer/sections/sectionSource.ts`
7. `features/viewer/sections/useSectionView.ts`
8. `features/viewer/sections/useSectionView.test.tsx`
9. новый `features/project-sources/sectionSource.test.tsx`

Изменение существующего теста только инициализирует managed opening через штатный
setProject вместо прямой записи project без binding. Все проверки переключения
режимов, числа native вызовов и восстановления результата сохранены.

## Доказательства и границы

Отдельная копия с ровно этими изменениями frontend поверх согласованного manual API
прошла **397 тестов / 121 файл**, production build, lint и i18n (1483 сообщения).
Остальные отслеживаемые frontend-исходники совпадают с рабочей основой; точный diff
прошёл `git apply --check`. 15 новых тестов: шесть face и девять section, включая
настоящий frontend API invoke-контракт с mock транспорта и отказ чужому STEP hash.
Эти проверки не являются выполнением OCCT или installed-app acceptance.

Новых пользовательских текстов и стилей нет. Кнопка папки, writer/permission UI,
history/import/export этим патчем не подключаются. Этап 4 остаётся незавершённым.
Общие frontend/scene/section файлы согласованы владельцем по §5 ТЗ перед применением.
