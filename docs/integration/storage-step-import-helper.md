# Reuse STEP import intent / Повторное использование STEP intent

Status: exact two-file patch prepared, not applied. Base `d1368cc`.
[Patch](storage-step-import-helper.patch).

## RU

Для отдельного переноса выбранного последнего STEP в новый FCP1-проект нужна
существующая чистая функция `conversion::import_document::source`. Она строит
тот же проверенный CAD IR ImportStep с SHA-256 asset reference; она не открывает
файлы, не выдаёт разрешения и не записывает проект. Патч меняет только visibility:
регистрацию модуля в `conversion.rs` и функцию в `conversion/import_document.rs`
с private/pub(super) на pub(crate). Алгоритм, проверки и тесты не меняются.

Owned transfer будет использовать выбранный bounded STEP source, новое project
identity и существующий common CAD executor. Старая БД 1.1 не открывается и не
мигрирует; история и параметрика старого проекта не объявляются восстановленными.
Полноценный transfer и реальная геометрия требуют отдельной реализации и CI.
Это не подключение UI и не изменение native/Cargo/permissions/версий.

Общие conversion-файлы требуют согласования §5 ТЗ. До разрешения этот точный
патч остаётся артефактом; owned source capability можно реализовывать независимо.

## EN

The exact patch only exposes the existing pure STEP ImportStep-document factory
within the crate: module registration in `conversion.rs` and `source` visibility
in `conversion/import_document.rs`. No algorithm, validation or test changes.
The function hashes supplied bytes and creates validated CAD IR; it grants no
permission, opens no files and performs no publication. The owned transfer will
use a bounded explicitly selected source, fresh project identity and the common
CAD executor. It will not open or migrate a genuine 1.1 database or claim restored
parametric history. Transfer implementation, real-worker checks and UI integration
remain incomplete. Native, Cargo, permissions and versions are untouched.
