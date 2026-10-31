# Чистая проекция каталога проектов

[Документация](index.md) · [English](../en/storage-catalog.md) · [Контракт](storage-contract.md)

`project_repository::catalog` — backend API для перечисления сохранённых managed
проектов без recovery и получения доступа на запись. Он пока не подключён к
`list_projects`, Dashboard или IPC. Пользовательский список продолжает прежний
маршрут; новая возможность не объявляется доступной кнопкой или настройкой.

## Контракт

`list_managed(state, CatalogLimits { max_entries, max_total_payload_bytes })`
возвращает `CatalogReport`: все выбранные строки и фактическую сумму JSON-байтов
готовых `Project`. Лимиты явно задаёт host; продуктовых defaults здесь нет.
Если строк больше разрешённого числа, весь вызов возвращает ошибку лимита,
а не успешный усечённый список. Ошибка общей SQL-базы также остаётся ошибкой.

| Entry | Данные | Значение |
| --- | --- | --- |
| `ready` | Полный сохранённый `Project` | Один чистый repository snapshot и recheck исходного сеанса |
| `unavailable` | Ограниченные index hints и issue | Настоящий проект не подменяется выдуманной моделью |

Issue различает `recovery_required`, `changed`, `unavailable` и `limit`. Index
hints не являются проверенным состоянием: невалидные ID и даты не становятся
authority. `rowKey` — строка `managed-index:{rowid}`, поэтому даже rowid выше
JavaScript `2^53` сохраняет уникальность. Недоступные записи не пропускаются;
здоровые остаются в ответе вместе с ними. Tokens/opening/leases не сериализуются.

## Технический маршрут и лимиты

Ограниченный SQL SELECT читает ID, имя, дату и длину payload, но не все payload
сразу. Каждая допустимая строка проходит `ProjectRepository::managed → capture
→ recheck` исходного snapshot. Нет `projects::get`, reconcile, writer acquire,
миграций, записи индекса или очистки pending journal. Это отдельные optimistic
проверки проектов, а не атомарный снимок всего каталога.

Предварительная длина raw payload используется консервативно: запись с большим
количеством пробелов может быть отклонена до более короткой canonical serialization.
Фактическая сериализация готового `Project` затем считается bounded writer без
ещё одного полного JSON-буфера; устаревшая SQL-подсказка размера не обходит бюджет.
Перед выходом выполняется recheck, клонируется только принятый `Project`.

Бюджет относится к сумме готовых Project JSON, не включает envelope/index hints
и не ограничивает peak memory. При гонке строки остаётся существующий bounded
repository capture/recheck working set (raw до 128 МиБ) плюс принятый clone.
Готовая запись содержит переписку, ревизии и файлы для совместимости будущего UI;
это не лёгкий metadata-only индекс и не доказательство скорости большого списка.

## Интеграция и проверка

Для UI нужен согласованный shared-патч с обработкой недоступных карточек,
сохранением рабочего workspace и явным действием recovery. Текущий Dashboard
также автоматически сохраняет thumbnails: замена backend list сама по себе не
сделает весь экран чистым. `PROJECT_CATALOG_LIMIT` и
`PROJECT_CATALOG_UNAVAILABLE` пока внутренние; перед отображением добавить RU/EN
переводы на границе UI, оставив технические details отдельно. Общий `Project`
и намеренный opener `projects::get` не изменялись.

Девять SQL-регрессий проверяют смешанные ready/pending/corrupt записи,
неизменность SQL/источников/journal, read-only без upgrade, лимиты, escaped JSON,
гонку размера, неправильные ID, close во время SQL await, ошибки базы и rowid
выше `2^53`. Команда:

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::catalog`

Наличие тестов не означает PASS: результат нового head фиксируется отдельно в CI.
Фикстуры временные и синтетические; реальный NAS/USB и пользовательская UI-приёмка
не выполнялись.
