# Программа хранения: проверка основы

[Документация](index.md) · [English](../en/storage-program.md) · [Снимки](project-snapshots.md)

Ответственный: wenlixxq; приёмка и слияние: lovlyDev. Основа локальной ветки
`codex/storage-week` — `abe2e054cb4b4f807aa89195825edd0e4e82c62a` (head PR №5
по приложенному ТЗ). Актуальные PR, замечания, CI и Issue №3 не подтверждены:
подключённый GitHub возвращает HTTP 403 `Your account was suspended`.
Версии, релизы, установка и область native/Cargo/worker исключены из изменений.

## Исправление сеанса общего снимка

Пользователь продолжает пользоваться существующими чтением вложений,
копированием и экспортом. Дополнительных действий и настроек нет.
`CommittedSnapshot` теперь удерживает исходный внутренний `ReadSession`:
он захватывается до SQL, проверяется после каждого SQL await и перед возвратом.
При повторной проверке сохраняется исходный token, а новый захват не даёт
старому ответу действительность. Close/reopen отклоняет старый ответ с
существующим переводимым `PROJECT_SNAPSHOT_CHANGED`; обновите проект и
повторите действие. Читатель не получает writer lease и не выполняет recovery.

| Потребитель | Маршрут | Граница |
| --- | --- | --- |
| Статус истории | capture_history_state → recheck_history_state | Исходный token и типизированный SQL-курсор; архив отдельно |
| Вложения | capture_project → ограниченное чтение/hash → recheck | Получает общий lifecycle-контроль |
| Копирование | capture_with_history → artifacts → recheck → persist | Запись назначения выполняется отдельно |
| CADPACK | capture_with_history → artifacts → recheck → publish | Публикация после optimistic recheck |
| Свойства STEP | capture_project → worker → recheck | Код native-потребителя не изменён |
| Preview | capture_project → worker → recheck | Последующий await cleanup не является атомарной доставкой |
| Сечения | capture_project → recheck и artifact checks | Код потребителя не изменён |
| Измерения | snapshot::read → project | Token теряется; нужен согласованный патч интеграции |

## Ограничения и следующая работа

Гарантия optimistic: состояние проверяется в определённые моменты, дальнейшее
изменение после последней проверки не блокируется. Путь пока не является OS
identity папки: подмена по тому же пути с теми же байтами не обнаруживается.
AccessRegistry индексируется ID, а не backend/location; acquire/status/ensure_write
выполняют файловые операции под общим mutex. Эти ограничения требуют отдельных
изменений перед готовностью следующего этапа. Внешняя папка ещё не является
самостоятельным backend. Этап 1 и программа целиком не объявлены завершёнными.

## Воспроизводимая проверка

Новые тесты находятся в `project_repository/legacy/session_tests.rs`:
close/reopen для project/archive, закрытие во время SQL capture и SQL recheck.
Пулы исчерпываются явно, future опрашивается до pending; sleeps не используются.
Проверяется отказ и неизменность логического источника; SQL-тесты также сравнивают
файлы. Существующие семь тестов статуса не дублируются.

Команда: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::legacy`.
Фикстуры синтетические; это не проверка настоящей геометрии, USB/NAS или установленного UI.
Результат исполнения фиксируется отдельно; наличие теста не означает PASS.

Локальная проверка 29 октября 2026: rustfmt и `git diff --check` прошли;
check:docs — 108 документов, check:i18n — 1149 сообщений,
check:migrations и check:version прошли (версия остаётся 1.3.0).
Rust-тесты не выполнились: после загрузки зависимостей компиляция остановилась
на `linker link.exe not found`. Штатный `scripts/windows-env.ps1` также
остановился: отсутствует `C:\Program Files (x86)\Windows Kits\10\Lib`.
CI своей ветки и строгий Clippy не подтверждены. Установщик не собирался.
