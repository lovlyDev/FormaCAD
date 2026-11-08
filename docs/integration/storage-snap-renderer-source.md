# Snap курсора и контекстного меню: исходное открытие и последний запрос

[English](storage-snap-renderer-source.en.md) · [Host API](../ru/storage-folder-host-api.md) · [Измерения](storage-measurement-renderer-source.md)

Статус: **применён после явного согласования всех девяти файлов владельцем**.
Issue #31 / PR #35, исполнитель wenlixxq, приёмка lovlyDev.
Основа `db3ce03d643fcc4adc2ed592fbab2ac4c3f4046e` содержит применённые 16 файлов
измерений. Точный [патч](storage-snap-renderer-source.patch) затрагивает девять путей
и не включает неподключённый face-section прототип.

## Поведение

Snap-запрос захватывает исходные project/opening/backend binding, revision, saved
program/programBase и SHA/size основного STEP. Его readonly FolderSourceRef передаётся
существующему exactSnap API. После await проверяются первоначальное открытие и
геометрический контекст; другой проект или папка с одинаковым revision ID не
принимает старый результат. Metadata-only refresh совместимой геометрии сохраняет
валидность работающего запроса, не меняя его исходную commit-ссылку; новая операция
захватывает свежий commit до своего await.

Общий контроллер выбирает последнее размещение курсора среди pointer- и pie-запросов.
Старый worker может закончить работу, но его результат, ошибка или finally не меняют
новые pending/error/position. Symbol identity не переиспользуется при переполнении
счётчика. Непрозрачный host token этим не создаётся; snapshot и request identity
остаются в RAM и не предоставляют writer permission.

Успешное завершение освобождает listener. Отображённая ошибка сохраняет только один
watcher до смены источника или нового запроса, чтобы ошибка не переходила в другое
открытие. Тело проекта целиком не клонируется: сравниваются исходные primitive поля.
Источник не открывается заново по пути или ID. Нативная очередь и cancellation API
не меняются; контроллер отменяет принятие результата, а не обещает остановку worker.

Pointer handler проверяет исходный отображаемый объект, unmount и номер размещения.
Результат имеет повторную проверку непосредственно перед установкой координат.
Контекстное меню захватывает edge/source при открытии и не выбирает другое ребро
или папку позднее. Команда world origin тоже вытесняет старый native snap. Оценочная
точка при обычной ошибке worker остаётся явно non-exact и допускается только пока
исходный контекст текущий. Смена контекста возвращает null вместо старой точки.

Исходный read-контекст захватывается до load await и после декодирования
привязывается к полученному объекту через WeakMap. Ссылки не помещаются в userData, GLB или JSON. Старый объект не читает новый глобальный
проект как свой. Folder-объект без этой привязки не заимствует активный folder source.
Viewport commands передают реальный объект в snap pie; проверяется и scene token
выбранного ребра. Старое меню не сбрасывает курсор другой папки командой world origin.
Отказ host со stale-source/provenance кодом возвращает null, без замены mesh estimate.

## Все 9 путей

Относительно `apps/desktop/src/`:

1. новый `features/snapping/snapRequest.ts`
2. `features/snapping/resolveSnap.ts`
3. новый `features/pie-menus/edgeSnapActions.ts`
4. `features/pie-menus/viewportPies.ts`
5. `features/snapping/cursor/Cursor3D.tsx`
6. новый `features/project-sources/snapSource.test.ts`
7. новый `features/project-sources/sceneRead.ts`
8. `features/model-apply/useCommittedScene.ts`
9. `features/viewer/view-commands/viewportCommands.ts`

Самостоятельная native edge-операция вынесена из смешанного модуля viewport pies.
Другие меню сохраняют прежние действия и тексты. Новых пользовательских строк,
стилей, Rust/native/Cargo/backend IPC/SQL/версий нет.

## Проверки и пределы

В отдельной копии текущей основы **только с этими девятью файлами** прошли
**382 теста / 119 файлов**, production build, lint и i18n (1483 EN/RU сообщения).
19 новых тестов проверяют original routing, equal-ID reopen/project change,
metadata continuation, stale pending/error/finally, scene close, свежий commit
следующего запроса, старое меню и world-origin supersession. Применимость exact
diff проверена. Это renderer-моки; они не являются новыми измерениями OCCT или
installed-app acceptance. Композитный прототип с face core не используется вместо точной проверки этих девяти путей.

Face/manual sections и writer/permission UI этим патчем не подключаются. Видимой
кнопки папки нет; этап 4 остаётся незавершённым. По §5 ТЗ общие snap/pie/cursor файлы
были согласованы владельцем до применения.
