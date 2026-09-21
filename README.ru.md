# Forma CAD

[English](README.md) · [Русский](README.ru.md) · [Скачать](https://github.com/lovlyDev/FormaCAD/releases) · [Документация](docs/ru/index.md)

Я создаю Forma как локальную CAD-среду, в которой ручное редактирование и AI работают с одной моделью. Приложение объединяет Rust-бэкенд, интерфейс React/Three.js и геометрический процесс Python/CadQuery.

**Версия 1.1.0.** В этом выпуске я обновил рабочее пространство и исправил запуск приложения с существующей базой проектов. Подробности — в [описании версии](docs/releases/1.1.0.md) и [плане развития](docs/ru/roadmap.md).

## Возможности

- Параметрические окружности, прямоугольники, выдавливание, перемещение и булевы операции; редактирование параметров и ограниченный CadQuery fallback.
- Локальные проекты, история ревизий, восстановление, сравнение моделей и импорт STEP.
- Экспорт STL, OBJ, GLB, 3MF и построенной STEP-геометрии; ракурсы, видимость тел, измерения и движение сборок.
- Интеграция Codex / Claude CLI с разрешениями и локальной проверкой геометрии.
- Русский и английский, светлая и тёмная темы, сохранение рабочего места и подписанные обновления.

## Установка

Установщики доступны в [GitHub Releases](https://github.com/lovlyDev/FormaCAD/releases/latest).

| Система | Пакет | Обновление |
| --- | --- | --- |
| Windows x64 | EXE-установщик | В приложении |
| macOS Apple Silicon / Intel | DMG | В приложении |
| Linux x64 | AppImage | В приложении |
| Linux x64 | DEB / RPM | Установка нового пакета |

Файлы появятся после успешной [релизной сборки](.github/workflows/release.yml). Подпись/notarization macOS и подпись обновлений — разные механизмы: [руководство по релизам](docs/ru/releases.md). Для AI-моделирования нужны отдельно установленные CLI и Python/CadQuery: [подготовка окружения](docs/ru/development.md). Для просмотра сохранённых моделей AI-аккаунт не нужен.

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

Подробности: [разработка](docs/ru/development.md), [архитектура](docs/ru/architecture.md), [данные и восстановление](docs/ru/data.md), [автоматизация релизов](docs/ru/releases.md), [участие в проекте](CONTRIBUTING.ru.md).

## Данные и приватность

Проекты, настройки и резервные копии находятся в каталоге данных приложения. Обновления сохраняют идентификатор и пути хранения. Перед установкой Forma сохраняет рабочее место и создаёт резервную копию SQLite. Ракурс, параметры отображения, панели и черновики привязаны к проекту. Подробнее: [хранение данных](docs/ru/data.md).

Проверка обновлений обращается к GitHub. AI-запросы передаются выбранному CLI-провайдеру при разрешённом действии. Сервер Forma не получает данные проектов. Подробнее: [безопасность](SECURITY.ru.md).

## Ссылки

[Репозиторий](https://github.com/lovlyDev/FormaCAD) · [Задачи](https://github.com/lovlyDev/FormaCAD/issues) · [Релизы](https://github.com/lovlyDev/FormaCAD/releases) · [Сборки](https://github.com/lovlyDev/FormaCAD/actions) · [Изменения версии](docs/releases/1.1.0.md)
