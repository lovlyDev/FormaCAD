# Чистая проекция каталога проектов

[Документация](index.md) · [English](../en/storage-catalog.md) · [Контракт](storage-contract.md)

`project_repository::catalog` — backend API для перечисления сохранённых managed
проектов без recovery и получения доступа на запись. IPC `list_projects` возвращает CatalogReport, а `read_project_snapshot` читает один проект без recovery. Dashboard использует полный report, включая unavailable-карточки; workspace/model-apply/events выполняют чистое чтение выбранного проекта.

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
Готовая запись содержит переписку, ревизии и файлы для совместимости UI;
это не лёгкий metadata-only индекс и не доказательство скорости большого списка.

## Интеграция и проверка

Managed-каталог в UI использует описанный ниже report. Ошибки лимита и недоступности переведены в RU/EN; технические details отделены. Общий Project и намеренный projects::get не изменены.

Девять SQL-регрессий проверяют смешанные ready/pending/corrupt записи,
неизменность SQL/источников/journal, read-only без upgrade, лимиты, escaped JSON,
гонку размера, неправильные ID, close во время SQL await, ошибки базы и rowid
выше `2^53`. Команда:

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::catalog`

Наличие тестов не означает PASS: результат нового head фиксируется отдельно в CI.
Фикстуры временные и синтетические; реальный NAS/USB и пользовательская UI-приёмка
не выполнялись.


## Первый IPC-контракт и оставшаяся интеграция

Host ограничивает ответ 1000 записями и 128 МиБ ready JSON; это не peak memory. `projects/catalog.rs` связывает чистый каталог и target-read с IPC, не меняя намеренный `projects::get`, persist или recovery. Target-read не выдаёт writer lease и не переносит session token в UI. Оставшиеся legacy Project[] callers сохраняют строгий compatibility adapter: частичный report вызывает ошибку, а не молчаливое скрытие строк. Сам Dashboard читает report напрямую.


## Каталог в Dashboard

Dashboard читает CatalogReport и показывает как ready-проекты, так и отдельные unavailable-карточки со статусом, ограниченными подсказками индекса и повтором чистого чтения. Строковый rowKey сохраняется без преобразования в число. Непроверенная запись не получает фиктивную модель, действия редактирования/экспорта/удаления или кнопку автоматического восстановления. Технические details раскрываются отдельно. Поиск включает имя/ID-подсказку недоступной строки.

Список больше не создаёт и не сохраняет thumbnails в фоне: используются уже сохранённые изображения либо стандартная иконка. Открытие карточки и прямого маршрута читает только выбранный проект через read_project_snapshot; ошибка чужой строки не мешает здоровому target-read. Во время чтения прямого маршрута прежняя сцена и её собственное пространство UI-настроек сохраняются на экране; редактирование модели блокируется. При ошибке активный workspace сохраняется и восстанавливается его URL; сброса сцены и перенаправления на главную нет. Общая generation-проверка и сравнение исходного workspace отбрасывают поздний ответ после другого выбора или локального обновления. Model-apply и revision events тоже используют target-read с сохранением прежних guards. Catalog cache отделён от Project[] snapshot cache; мутации и события инвалидируют оба через общий helper. Закрепление и переименование сначала читают текущий target, а затем вызывают прежний writer API.

Это managed-каталог, не самостоятельный внешний folder writer. Явный recovery UI остаётся отдельной интеграцией; Retry ничего не исправляет автоматически.

Проверка UI 31 октября 2026: сборка и lint прошли; 227 unit-тестов в 79 файлах прошли с одним worker. Параллельный прогон имел таймаут существующего Sketch2d-теста; последовательный повтор проверил весь набор. [Браузерный отчёт](../storage-catalog/ui-check.json) подтверждает четыре статуса, поиск/retry/details, строковые rowKey и RU/EN с обеими темами на одном mount без изменения тестового project storage. Проверены снимки [RU dark](../storage-catalog/ru-dark.png) и [EN light](../storage-catalog/en-light.png). Это настоящий Dashboard с синтетическими props, не native IPC/recovery и не проверка сохранения CAD-сцены при смене темы.
