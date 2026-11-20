# FCP1 copy and recent folders integration proposal

## English

Review the [follow-up patch](storage-folder-copy-recents.patch) and its
[26 paths](storage-folder-copy-recents.paths). Its SHA-256 is
`18058541d90e522806f06c3e42f359fe07cf20483b924aa61e4d1e2485b0cfab`.
The runtime base is `38205adf5217bc60d0c84842b6bb37f766abb6cb` with the
[workspace proposal](storage-folder-workspace.md) applied first. The workspace
patch keeps its original SHA-256. Together the two patches change 62 paths.

The File menu offers an independent copy of the active FCP1 project. The host
captures the original project and commit before the native empty-folder chooser
and preserves the original bytes and history. The copy has a fresh identity.
The menu also exposes the existing source editor for a writable FCP1 opening.

The dashboard displays paths, open times and pins from the disposable index.
A recent location seeds the native picker. Only confirming the picker opens a
folder. The setting to remember opened folders affects future recordings and
preserves existing entries. Index errors do not block direct folder selection.

The combined local review passed 483 frontend tests, TypeScript, ESLint,
production build, localization, documentation links, and Rust formatting.
The isolated workflow compiles Tauri and runs host tests with both patches
applied. Native desktop runtime execution remains a separate check.

## Русский

Для согласования подготовлены [дополнительный патч](storage-folder-copy-recents.patch)
и [список 26 файлов](storage-folder-copy-recents.paths). SHA-256 патча —
`18058541d90e522806f06c3e42f359fe07cf20483b924aa61e4d1e2485b0cfab`.
Основа — `38205adf5217bc60d0c84842b6bb37f766abb6cb` после применения
[основного патча рабочей области](storage-folder-workspace.md). Хеш основного
патча сохраняется. Вместе оба патча меняют 62 файла.

В меню «Файл» появляется сохранение независимой копии активного проекта
FCP1. Приложение фиксирует исходный проект и коммит перед системным выбором
пустой папки. Исходные байты и история сохраняются, копия получает новый
ID. Меню также открывает редактор исходного кода в доступной для записи папке.

Главный экран показывает пути, время открытия и закрепления из отдельного
индекса. Запись недавнего пути задаёт начальный каталог в системном диалоге.
Проект открывается после подтверждения выбора. Настройка запоминания влияет
на новые записи и сохраняет прежние. Ошибка индекса не мешает выбрать папку
напрямую.

Локально прошли 483 frontend-теста, TypeScript, ESLint, production-сборка,
переводы, ссылки документации и форматирование Rust. Изолированный workflow
компилирует Tauri и выполняет backend-тесты с обоими применёнными патчами.
Запуск настольного сценария проверяется отдельно.
