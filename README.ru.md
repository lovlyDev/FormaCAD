# Forma CAD

[Топологические ссылки на операции](docs/ru/topology-references.md): точный выбор поддерживаемых рёбер после переноса, поворота и отражения.

[Host-транзакции модели](docs/ru/model-apply-transactions.md) · [Хранение после commit](docs/ru/postcommit-persistence.md): проверки результата, отмена, восстановление и уведомления.

[English](README.md) · [Русский](README.ru.md) · [Скачать](https://github.com/lovlyDev/FormaCAD/releases) · [Документация](docs/ru/index.md)

Я создаю Forma как локальную CAD-среду, в которой ручное редактирование и AI работают с одной моделью. Приложение объединяет Rust-бэкенд и интерфейс React/Three.js. Релизы по тегу включают C++/OpenCascade для документов CAD IR v2 на Windows, macOS и Linux; Python/CadQuery остаётся для старых моделей.

**Объединённый релиз 1.2.6: Windows-установщик собран, проверен и подписан для updater.** [Windows EXE](https://github.com/lovlyDev/FormaCAD/releases/download/v1.2.6/Forma_1.2.6_x64-setup.exe) — 19 017 594 байта; [страница релиза](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.2.6). Полная 2.0 не готова. См. [доказательства и ограничения](docs/ru/status.md), [заметку релиза](docs/releases/1.2.6.md) и [процедуру выпуска](docs/ru/releases.md).

## Возможности

- CAD IR v2 с именованными параметрами и телами, историей операций, подавлением, откатом, многоугольными эскизами со связями, точными операциями OpenCascade и ограниченным режимом совместимости CadQuery.
- Локальные проекты, история ревизий, восстановление, сравнение моделей и импорт STEP.
- Экспорт STL, OBJ, GLB, 3MF и построенной STEP-геометрии; ракурсы, видимость тел, измерения и движение сборок.
- Интеграция Codex / Claude / Custom CLI с разрешениями и локальной проверкой геометрии.
- Русский и английский, светлая и тёмная темы, сохранение рабочего места и подписанные обновления.

## Установка

Последняя **опубликованная** версия доступна в [GitHub Releases](https://github.com/lovlyDev/FormaCAD/releases/latest). [Windows EXE 1.2.6](https://github.com/lovlyDev/FormaCAD/releases/download/v1.2.6/Forma_1.2.6_x64-setup.exe) проверен и подписан для updater; см. [ограничения](docs/ru/status.md).

| Система | Пакет | Обновление |
| --- | --- | --- |
| Windows x64 | EXE-установщик | В приложении |
| macOS Apple Silicon / Intel | DMG | В приложении |
| Linux x64 | AppImage | В приложении |
| Linux x64 | DEB / RPM | Установка нового пакета |

Файлы появятся после успешной [релизной сборки](.github/workflows/release.yml). Подпись/notarization macOS и подпись обновлений — разные механизмы: [руководство по релизам](docs/ru/releases.md). Для AI-моделирования нужен отдельно установленный CLI; релизные сборки включают нативное CAD-ядро. Python/CadQuery остаётся необязательным для старых моделей: [подготовка окружения](docs/ru/development.md). Для просмотра сохранённых моделей AI-аккаунт не нужен.

## Разработка

Установите Node.js 22+, Rust stable и [системные зависимости Tauri](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm run desktop
```

Предпросмотр в браузере: `npm run dev`. В браузере недоступны запуск локальных CLI и установка обновлений.

```sh
npm run check:version
npm run check:migrations
npm run check:i18n
npm run check:docs
npm run lint
npm test
npm run test:e2e
npm run build
```

Начните с [индекса документации](docs/ru/index.md). Подробные руководства: [работа в Forma](docs/ru/using-forma.md), [проекты](docs/ru/projects.md), [моделирование](docs/ru/modeling.md), [эскизы](docs/ru/sketches.md), [AI и разрешения](docs/ru/ai.md), [просмотр и файлы](docs/ru/viewer-and-files.md), [настройки и обновления](docs/ru/settings-and-updates.md), [данные и восстановление](docs/ru/data.md), [архитектура](docs/ru/architecture.md), [карта кода](docs/ru/internals.md), [разработка](docs/ru/development.md), [автоматизация релизов](docs/ru/releases.md) и [участие в проекте](CONTRIBUTING.ru.md). Новые возможности требуется документировать на обоих языках по [правилам документации](docs/ru/documentation.md).

## Данные и приватность

Проекты, настройки и резервные копии находятся в каталоге данных приложения. Обновления сохраняют идентификатор и пути хранения. Перед установкой Forma сохраняет рабочее место и создаёт резервную копию SQLite. Ракурс, параметры отображения, панели и черновики привязаны к проекту. Подробнее: [хранение данных](docs/ru/data.md).

Проверка обновлений обращается к GitHub. AI-запросы передаются выбранному CLI-провайдеру при разрешённом действии. Сервер Forma не получает данные проектов. Подробнее: [безопасность](SECURITY.ru.md).

## Ссылки

[Репозиторий](https://github.com/lovlyDev/FormaCAD) · [Задачи](https://github.com/lovlyDev/FormaCAD/issues) · [Опубликованные релизы](https://github.com/lovlyDev/FormaCAD/releases) · [Сборки](https://github.com/lovlyDev/FormaCAD/actions) · [Все заметки об изменениях](docs/ru/changelog.md)

[Вспомогательные линии эскиза](docs/ru/sketch-construction.md) доступны в локальной разработке.

Локальный Sketcher: [отверстия и контуры](docs/ru/sketch-profiles.md), [редактирование и связи](docs/ru/sketch-constraints.md), [параметры](docs/ru/cad-parameters.md), [диагностика](docs/ru/sketch-diagnostics.md).


[Локальные изменения 1.2.6](docs/releases/1.2.6.md): редактор эскизов, связи параметров и адаптивная сетка; [руководство](docs/ru/sketches.md).

[Типизированные правки AI](docs/ru/command-api.md): изменения общих параметров и связей одной проверяемой ревизией.

[Проверка AI-кандидата ](docs/ru/ai-candidate-review.md).

[Проверенный выбор CAD-геометрии для AI](docs/ru/ai-selection-context.md).

[История модели, хранилище и нативные операции](docs/ru/project-history.md) · [Хранилище](docs/ru/project-storage-v2.md) · [Паритет](docs/ru/modeling-parity.md)

[Доступ к проекту](docs/ru/project-access.md) · [Операции поверх STEP](docs/ru/imported-step-features.md)

[Точные сечения модели](docs/ru/model-sections.md) · [Очередь тяжёлых CAD-задач](docs/ru/cad-task-queue.md)

[Сечение по выбранной CAD-грани](docs/ru/face-sections.md): знаковые смещения, проверенные плоскости и пересечение всего STEP.

[Точные измерения между элементами](docs/ru/pair-measurements.md): минимальные расстояния выбранной пары CAD-элементов и углы нормалей/прямых одного сохранённого тела.

[Точные измерения выбранной геометрии](docs/ru/reference-measurements.md): входит в объединённый исходный код 1.2.6; Windows-пакет проверен.

[Измерения круговых рёбер](docs/ru/circular-measurements.md): точные авторские радиус и диаметр; входят в объединённый исходный код 1.2.6.

[Чистые снимки проектов](docs/ru/project-snapshots.md): согласованное чтение проекта и истории без неявного восстановления.


[Совместная работа с двух ПК](docs/ru/collaboration.md): исходники через Git, пользовательские данные отдельно.
