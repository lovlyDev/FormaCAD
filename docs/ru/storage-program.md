# Программа хранения: проверка основы

[Документация](index.md) · [English](../en/storage-program.md) · [Снимки](project-snapshots.md)

Ответственный: wenlixxq; приёмка и слияние: lovlyDev. Основа локальной ветки
`codex/storage-week` первоначально — `abe2e054cb4b4f807aa89195825edd0e4e82c62a`.
PR №5 подтверждён объединённым через API; 29 октября принят актуальный `main`
`4834e1020f755d4a158a105fc63965315d9c78c1` обычным merge в своей ветке.
Локально авторизованный аккаунт wenlixxq имеет рабочий API-доступ;
ошибка 403 отдельного подключённого аккаунта не относится к этой авторизации.
Программа закреплена в [Issue №27](https://github.com/lovlyDev/FormaCAD/issues/27),
этапы — №28–34; область Issue №3 прочитана и остаётся исключённой.
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
| Описание и список ревизий | listProjects → list_projects → reconcile | Ещё не переведены: raw SQL, возможный recovery; lifecycle-регрессии нет |
| Структурный diff ревизий | compare_ir_revisions → ProjectRepository → capture → recheck | Согласованный патч применён; отдельные stale/purity тесты ожидают CI |
| Вложения | capture_project → ограниченное чтение/hash → recheck | Получает общий lifecycle-контроль |
| Копирование | capture_with_history → artifacts → recheck → persist | Запись назначения выполняется отдельно |
| CADPACK | capture_with_history → artifacts → recheck → publish | Публикация после optimistic recheck |
| Свойства STEP | capture_project → worker → recheck | Код native-потребителя не изменён |
| Preview | capture_project → worker → recheck | Последующий await cleanup не является атомарной доставкой |
| Сечения | capture_project → recheck и artifact checks | Код потребителя не изменён |
| Измерения | snapshot::read → CommittedSnapshot → worker → recheck | Исходный lifecycle и отдельное сравнение geometry seals; metadata-only save сохраняет запрос |

Коррекция измерений удерживает исходный lifecycle до/после geometry recapture,
сохраняя существующее поведение metadata-only save. Подробности и границы
native-проверок — [в согласовании](../integration/storage-measurement-session.md).
Новый workflow `Storage native consumers` исполняет три host suites; наличие
workflow ещё не означает успешный результат исправленного head.

## Ограничения и следующая работа

Гарантия optimistic: состояние проверяется в определённые моменты, дальнейшее
изменение после последней проверки не блокируется. Сеанс теперь удерживает OS
identity корня и существующих каталогов источника: Windows volume/file ID,
Unix device/inode. Открытые read handles предотвращают повторное использование
удерживаемой identity и не дают writer lease. Путь сравнивается вместе с identity;
rename, подмена по тому же пути и появление ранее отсутствовавшего каталога
отклоняют старый снимок. Symlink/reparse/junction в корне и цепочке пути запрещены.
Файловая система без поддерживаемого Windows file ID отклоняется, без fallback
на даты. Сохранение файловых handles не является доказательством гарантий NAS.
AccessRegistry индексируется ID, а не backend/location. Эти ограничения требуют отдельных
изменений перед готовностью следующего этапа. Внешняя папка ещё не является
самостоятельным write backend. Проверки основы этапа 1 завершены на SHA ниже;
приёмка и слияние остаются у lovlyDev. Программа целиком не завершена.

## Воспроизводимая проверка

Реестр использует короткую блокировку общей карты только для получения
`Arc<Mutex<ProjectState>>`. IO, получение OS lease и owner JSON защищены отдельным
mutex проекта. Медленный источник не удерживает общую карту. Сеансы, epoch и
Weak writer guard остаются в состоянии проекта; release сохраняет lease, пока
есть in-flight WriteGuard, и не удаляет lock-файл.
Новые `project_access/registry_tests.rs` проверяют работу другого проекта при
удерживаемом gate первого, release/reacquire с живым guard и transient write.
Дополнительная регрессия статуса истории проверяет закрытие во время SQL recheck.
Проверки исходного token выполняются после обоих SQL await захвата курсора.

`project_repository/location/tests.rs` проверяет неизменённые/отсутствующие пути,
подмену листа/родителя/корня, traversal, обычные файлы и платформенные ссылки.
Два теста общего снимка проверяют подмену каталога с неизменным SQL и одинаковые
project IDs/payload в двух разных корнях. Они прошли Quality на `1ad4937`;
последующие изменения требуют своего прогона.

Проверка записи истории выделена в чистый `project_history::prepare_commit`.
Она возвращает закрытый тип с проверенным cursor/event; SQL writer использует
ту же функцию внутри своей транзакции. Правила append, metadata, undo/redo,
неизменяемых ревизий и проверки исходного PreparedHistory сохранены. Это основа
адаптера, а не новый маршрут записи: внешняя папка пока не подключена.
Восемь тестов проверяют правила и SQL state/event с rollback без изменения источника.

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
check:migrations и check:version прошли на первоначальной основе 1.3.0.
После получения main версия 1.5.1 унаследована от владельца; задачная правка
не меняет версионные файлы относительно main. Проверки новой основы — отдельно.
Rust-тесты не выполнились: после загрузки зависимостей компиляция остановилась
на `linker link.exe not found`. Штатный `scripts/windows-env.ps1` также
остановился: отсутствует `C:\Program Files (x86)\Windows Kits\10\Lib`.
CI своей ветки и строгий Clippy не подтверждены. Установщик не собирался.

## Проверенная основа этапа 1

[Quality 40869695219](https://github.com/lovlyDev/FormaCAD/actions/runs/40869695219)
успешен на head `1ad4937569edb2aaf86ba3f7407b48c2f7b6cb34`: Windows, Linux,
macOS ARM/Intel, frontend, CAD Python и security. Rust library Windows/Linux:
249 passed, 0 failed, 3 ignored (benchmark и служебные helpers). Strict Clippy
и fmt прошли. Frontend: 215 тестов / 75 файлов; browser 45 passed / 35 skipped
без нативных fixtures, поэтому installed/native UI этим не подтверждён.
[Storage comparison 40869695213](https://github.com/lovlyDev/FormaCAD/actions/runs/40869695213)
на том же head прошёл strict Clippy с `native-worker-client` и benchmark.

Настоящий Linux worker отдельно проверен в
[Quality 40868152101](https://github.com/lovlyDev/FormaCAD/actions/runs/40868152101),
job 123428058352, head `147a0dd05e1a15c94c7656b9569eca4f028b795c`:
292 library passed / 0 failed / 3 ignored и 11 worker passed. Включена новая
`close_and_reopen_after_real_worker_rejects_unchanged_measurement_source`.
Шесть согласованных файлов измерений между `147a0dd` и `1ad4937` идентичны;
это проверка соответствующего маршрута, а не всего более позднего head.
В этом старом Quality другие jobs падали на уже исправленном benchmark lint;
весь старый run не объявляется зелёным.

После этой основы этап 2 продолжается; локальные правки SQL-компаратора ещё
не входят в указанное подтверждение. USB/NAS, полный новый folder write маршрут,
поздние pair/face ответы и установленный UI остаются отдельными gates.
