# FCP1 STEP transfer integration proposal

## English

Review the [patch](storage-step-transfer-ui.patch) and [15 paths](storage-step-transfer-ui.paths).
The SHA-256 is `49f96915a5f98a4a1871a57509d1bef84a9f6843186e8c4db45dcd8f87ae3e0b`.
Use base `7a21189e69a73b9ab188af88bef1bb13cf4a0452` with the
[workspace](storage-folder-workspace.md) and [copy and recents](storage-folder-copy-recents.md)
patches applied in that order. Together the three proposals touch 65 paths.

The New Project dialog offers **Transfer STEP to folder** when the native CAD
worker is available. The host prompts for a STEP/STP file, captures it read-only,
then prompts for an empty destination. The renderer supplies the project name,
agent and localized import description. The new FCP1 folder contains a fresh
project and one imported geometry revision in millimeters. Original files stay
untouched. STEP cannot supply the parametric editing history.

The isolated workflow checks hashes and path manifests, applies all three
patches, builds and tests the frontend, and compiles the Tauri handlers with
and without the native worker feature. The real-worker backend test and
the desktop picker flow are separate verification steps.

## Русский

Для согласования подготовлены [патч](storage-step-transfer-ui.patch) и
[список 15 путей](storage-step-transfer-ui.paths). SHA-256 —
`49f96915a5f98a4a1871a57509d1bef84a9f6843186e8c4db45dcd8f87ae3e0b`.
На основу `7a21189e69a73b9ab188af88bef1bb13cf4a0452` сначала применяются
[патч рабочей области](storage-folder-workspace.md) и
[копирование и недавние](storage-folder-copy-recents.md). В сумме три
предложения затрагивают 65 файлов.

В диалоге нового проекта действие **Перенести STEP в папку** доступно при
наличии нативного CAD worker. Приложение предлагает выбрать файл STEP/STP,
открывает его только для чтения и затем предлагает выбрать пустую папку.
Интерфейс передаёт название проекта, агента и локализованное описание
импорта. В новой папке FCP1 создаётся новый проект с одной ревизией
импортированной геометрии в миллиметрах. Исходные файлы сохраняются.
Параметрическая история недоступна из STEP.

Изолированный workflow проверяет хеши и перечни путей, применяет три
патча, собирает и тестирует интерфейс и компилирует Tauri-обработчики
с нативным worker и без него. Проверка backend с настоящим worker и
проверка диалогов в настольном приложении проходят отдельно.
