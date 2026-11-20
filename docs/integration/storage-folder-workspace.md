# FCP1 workspace integration proposal

## English

Review the [patch](storage-folder-workspace.patch) and its [49 paths](storage-folder-workspace.paths).
The runtime base is `305d962cda34a133b87bc77dc89d2afe914555fa`. The patch SHA-256 is
`4437fdffbcf21cc35ac59fe64a7e0347bcfba0bd8696b1ff63e8046fd2b4796d`.
This consolidated proposal includes the previously submitted write and commit
observation IPCs. Apply it as a unit to the base with both proposals unapplied.

The dashboard and New Project dialog use native pickers for opening an existing
FCP1 folder or creating a blank project in an empty folder. The editor and
history controls share the selected opening, commit and one-use write approval.
Uncertain writes can be checked by explicit commit observation. The original
managed routes are retained. All additions to the interface have RU/EN strings.

The proposed shared scope includes Tauri registration, model/history IPC,
App and dialogs, and the renderer types, actions and tests listed above.
The review workflow applies the exact patch inside the runner. It verifies
the hash and paths, frontend build and tests, Rust formatting, Clippy and
folder host tests. The local check passed 474 frontend tests, TypeScript,
ESLint, localization, documentation links and Rust formatting. The complete
desktop scenario with the real CAD worker still needs runtime verification.

## Русский

Для согласования подготовлены [патч](storage-folder-workspace.patch) и
[список 49 файлов](storage-folder-workspace.paths). Основа —
`305d962cda34a133b87bc77dc89d2afe914555fa`, SHA-256 патча —
`4437fdffbcf21cc35ac59fe64a7e0347bcfba0bd8696b1ff63e8046fd2b4796d`.
Патч включает ранее предложенные IPC записи и чтения результата коммита.
Он применяется целиком поверх основы, где оба предложения ещё не применены.

Главный экран и диалог нового проекта используют нативный выбор существующей
папки FCP1 или пустой папки для нового проекта. Редактор и история используют
исходное открытие, коммит и одноразовое разрешение. Неопределённую запись
можно проверить явным запросом состояния коммита. Маршруты управляемого
хранилища сохраняются. Новые тексты доступны на русском и английском.

Объём согласования включает регистрацию Tauri, IPC модели и истории, App,
диалоги и перечисленные типы, действия и тесты интерфейса. Проверка применяет
патч в отдельном checkout CI с контролем хеша и путей, сборкой и тестами
интерфейса, форматированием Rust, Clippy и тестами хранилища. Локально прошли
474 frontend-теста, TypeScript, ESLint, переводы, ссылки документации и
форматирование Rust. Сценарий в настольном приложении с настоящим CAD worker
ещё требует проверки.
