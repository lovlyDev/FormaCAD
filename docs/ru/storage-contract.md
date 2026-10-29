# Контракт чтения репозитория

[Документация](index.md) · [English](../en/storage-contract.md) · [Форматы](storage-formats.md)

`project_repository::backend::ProjectRepository` явно выбирает managed AppState/ID
либо выбранную папку v2. Нет глобального current backend. Это внутренний API,
он ещё не заменяет пользовательский open/save маршрут и не даёт writer lease.

| Операция | Managed | Folder v2 |
| --- | --- | --- |
| managed / folder | Проверка ID/пути без открытия записи | Чистое открытие metadata и retained OS identity |
| capture | SQL project, journal, папка, исходный ReadSession | Manifest/project metadata, opening token, OS identity |
| capture_with_history | Архив из того же SQL снимка | Manifest уже связывает SHA архива; байты читаются лениво |
| read_asset | Только точный descriptor, size/hash, исходный recheck | Bounded asset, size/hash, manifest/metadata, исходный opening |
| read_history | Только после capture_with_history | Явное чтение/проверка связанного архива |
| recheck | Исходный handle/token, SQL/folder state | Исходный handle/opening, identity, manifest и metadata |

`RepositorySnapshot` не сериализуется, его authority и варианты backend закрыты;
публично доступен неизменяемый `project()`. Нельзя принять снимок из другого handle,
даже с тем же ID. Managed archive возвращается как borrowed `Cow<[u8]>`, без
ненужной второй полной копии. Чтение архива из metadata-only SQL snapshot
отклоняется `HISTORY_INVALID_STATE`; свежий token не узаконивает старый результат.

`FolderRepository` удерживает отдельный opening с непрозрачным UUID и active flag.
Close и Drop необратимо инвалидируют его снимки. Reopen не переиспользует token;
одинаковые IDs в разных папках не смешиваются. Получение metadata не читает все
assets/историю и не создаёт файлов, lock, index, миграций или recovery. Повреждённый
asset отклоняется при явном чтении, metadata не объявляет его исправным.

Регрессии `project_repository/{folder,backend}/tests.rs` проверяют независимость
расположений/backend/открытий, close/reopen/drop, подмену папки, manifest drift,
проверки descriptor/hash, отсутствие изменений и закрытие во время SQL await.
Пока требуется CI актуального SHA. Это optimistic capture/recheck, не блокировка
внешнего писателя после проверки, не доказательство NAS/USB или installed UI.

Prepare/publish, writer lease, явное recovery и независимая запись новой модели
остаются следующей частью программы. Их нельзя получить из read token. Для
будущей записи выделен существующий типизированный history validator; это не
разрешение обходить проверки CAD/permissions или напрямую обновлять SQL.
