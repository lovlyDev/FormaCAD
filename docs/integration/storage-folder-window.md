# Окно и чтение самостоятельной папки — точный патч

[English](storage-folder-window.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **применён после явного разрешения lovlyDev «РАЗРЕШАЮ»** 4 ноября 2026.
Согласованы все восемь путей; компиляция и runtime-проверки применённого кода
ещё ожидаются. Основа патча — `61e9b0006b02eb239966f59a02a86065a9e0c619`.
Ответственный за патч — wenlixxq, приёмка — lovlyDev, Issue #31 / PR #35.

## Контракт

Патч подключает существующий FCP1 reader к времени жизни окна. Он не добавляет
кнопку, запись модели, миграцию или обещание готового пользовательского сценария.
Следующее подключение UI должно перевести технические ошибки на границе показа
на RU/EN и передавать ключ источника во все связанные модельные операции.

`open_project_folder()` получает окно от Tauri, закрепляет его scope **до**
ожидания системного выбора папки и открывает только явно выбранную папку.
Возвращает `null` при отмене либо `{sourceId, commitId, project}`. Ключ не является
writer grant и не сохраняется в app index. Путь из UI не принимается; копия
проекта в AppData и строка в управляемой SQLite не создаются.

Существующий `read_project_snapshot` сохраняет managed-вызов `{id}`. Для папки
он принимает `{id, sourceId, expectedCommitId}`: оба дополнительных поля
обязательны вместе. Проверяются исходное окно/открытие, project ID, commit и
неизменность исходного снимка после копирования ответа. Неверный ключ не приводит
к managed fallback. Отдельное дублирующее IPC для чтения не добавляется.

`close_project_source({sourceId})` закрывает только источник вызывающего окна.
`WindowEvent::Destroyed` отзывает весь scope. Запоздалый выбор папки использует
старый закрытый scope и получает отказ. Повторное использование label новым
окном создаёт новый scope; старые ключи не оживают. Максимум 16 scopes, по 16
готовых/pending источников. Map mutex не удерживается во время диалога, await,
SQLite или дискового IO. Drop общего host state также отзывает все scopes.

На неподдерживаемых FCP1-платформах команды компилируются, но открытие/чтение
получает `PROJECT_FOLDER_UNSUPPORTED`. Поддержка формата и macOS не расширяется
этим патчем. Cargo, native, protocol, permissions, схема SQL и версии неизменны.

## Восемь точных путей

- `apps/desktop/src-tauri/src/lib.rs`: managed state, Destroyed hook, две команды.
- `apps/desktop/src-tauri/src/projects.rs`: необязательные поля существующего IPC.
- `apps/desktop/src-tauri/src/project_repository/mod.rs`: регистрация модуля.
- `apps/desktop/src-tauri/src/project_repository/window_sources/mod.rs`: lifecycle/read facade.
- `apps/desktop/src-tauri/src/project_repository/window_sources/supported.rs`: карта scopes.
- `apps/desktop/src-tauri/src/project_repository/window_sources/ipc.rs`: системный picker/close.
- `apps/desktop/src-tauri/src/project_repository/window_sources/supported/tests.rs`: настоящая папка, чужое окно, ожидаемый commit/ID, закрытие и повторный label, drop.
- `.github/workflows/storage-benchmark.yml`: явный запуск новой регрессии с проверкой одного PASS.

## Проверки и ограничения

Точный diff: [storage-folder-window.patch](storage-folder-window.patch).
Проверены применимость `git apply --check` и rustfmt подготовленных Rust-файлов.
Патч применён, но ещё не скомпилирован; наличие теста не является PASS.
После применения обязательны strict Clippy, новая регрессия, существующий
folder packet и Quality на точном SHA. Системный диалог и Destroyed hook требуют
проверки настоящего desktop. UI, model/geometry routing, recent index и дальнейшие
этапы программы остаются отдельными незавершёнными критериями.
