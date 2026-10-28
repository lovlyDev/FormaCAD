# Сценарии проверки, выпуска и сопровождения

[Документация](index.md) · [English](../en/maintainer-scripts.md) · [Разработка](development.md) · [Релизы](releases.md)

Здесь описаны вспомогательные сценарии, которые раньше жили вне репозитория (личные `.cmd`/`.ps1` и временные скрипты отдельных сессий). Теперь они хранятся в `scripts/` в обобщённом виде: без абсолютных путей и без секретов. Ключи, `.pem`, `updater.key`, `.env` и токены по-прежнему хранятся только вне Git (`.secrets/` и `.local/` исключены).

## Сводка

| Сценарий | Назначение | Кто запускает |
| --- | --- | --- |
| [`scripts/windows/verify.ps1`](../../scripts/windows/verify.ps1) | Все проверки перед PR одной командой; по флагу — сборка Windows-установщика | любой участник, AI-агент |
| [`scripts/verify-updater-signatures.mjs`](../../scripts/verify-updater-signatures.mjs) | Проверка подписей обновлений (`*.sig`) открытым ключом из `tauri.conf.json` и сверка `SHA256SUMS` | ответственный за выпуск |
| [`scripts/maintainers/github-app-admin.mjs`](../../scripts/maintainers/github-app-admin.mjs) | Обслуживание GitHub App «FormaCAD Plugins»: установки, доставки webhook, повторная доставка, смена секрета | администратор каталога |
| [`scripts/maintainers/site-docs.py`](../../scripts/maintainers/site-docs.py) | Перенос руководств по плагинам в статьи сайта formacad.ru | администратор сайта |

## `scripts/windows/verify.ps1`

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify.ps1
```

Порядок: проверка версий → `check:i18n` → `check:docs` → `check:migrations` → ESLint → typecheck и сборка фронтенда → Vitest → `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test --lib`. На первой ошибке сценарий останавливается и пишет имя шага и код выхода. Перед Rust-шагами он подключает `scripts/windows-env.ps1`, который выбирает полный x64-тулчейн MSVC и Windows SDK.

Параметры:

- `-Install` — сначала `npm ci`.
- `-CargoTargetDir <папка>` — общая папка сборки Cargo, например одна на несколько worktree: повторная сборка OCCT-зависимых crate занимает много времени.
- `-TempDir <папка>` — `TMP`/`TEMP` для этого запуска, если на системном диске мало места.
- `-Installer -OcctRoot <папка>` — после проверок собрать установщик через `scripts/build-native-windows.ps1`. OCCT собирается `scripts/build-occt-windows.ps1` в `.local/occt-build`.
- `-UpdaterKeyFile <файл>` — ключ подписи обновлений, например `.secrets/updater.key`. Ключ читается только в память процесса, не печатается и не записывается. Без параметра используется переменная `TAURI_SIGNING_PRIVATE_KEY`; без неё установщик собирается без подписи обновления (сценарий предупреждает).

Нативные тесты worker и C++ (`cargo test --features native-occt --test native_worker`, `ctest` в `cad-core`) и e2e Playwright запускаются отдельно, когда меняются ядро или интерфейс: см. [разработку](development.md).

Ограничения: только Windows x64. Сценарий не устанавливает приложение и ничего не публикует.

## `scripts/verify-updater-signatures.mjs`

```bash
node scripts/verify-updater-signatures.mjs .release-artifacts
```

Для каждого `<файл>.sig` в папке (рекурсивно) сценарий проверяет подпись minisign/Ed25519 открытым ключом `plugins.updater.pubkey` из `apps/desktop/src-tauri/tauri.conf.json`. Поддерживаются оба варианта: подпись самих байтов и подпись хеша BLAKE2b-512. Затем, если есть `SHA256SUMS`, сверяет хеши всех перечисленных файлов. Код выхода 1 при любой ошибке. Вторым аргументом можно передать другой `tauri.conf.json`, например из ветки выпуска.

Порядок выпуска: workflow Release собирает установщики → `scripts/release-artifacts.mjs collect` и `merge` строят `latest.json` и `SHA256SUMS` → этот сценарий проверяет загруженные артефакты перед публикацией. Публикация выполняется только по прямому указанию пользователя (см. [AGENTS.md](../../AGENTS.md) и [релизы](releases.md)).

## `scripts/maintainers/github-app-admin.mjs`

```bash
GITHUB_APP_ID=<id> GITHUB_APP_PRIVATE_KEY_PATH=<путь к .pem вне Git> node scripts/maintainers/github-app-admin.mjs installations
```

Команды: `installations`, `deliveries [n]`, `redeliver <id доставки>`, `set-secret <файл>`. Сценарий подписывает краткоживущий JWT приложения (5 минут) закрытым ключом из указанного файла и обращается к GitHub REST API. Используется, когда каталог не получил событие релиза плагина: посмотреть доставки, повторить нужную и при смене секрета webhook обновить его в настройках App. Тот же секрет нужно задать на сервере каталога.

Сам сервер каталога и аккаунтов — отдельный репозиторий [FormaCAD-Server](https://github.com/lovlyDev/FormaCAD-Server). Сценарии развёртывания и диагностики сервера (контейнеры, `.env`, сертификаты) живут там и сюда не переносятся.

## `scripts/maintainers/site-docs.py`

```bash
python scripts/maintainers/site-docs.py <папка контента сайта> <sha коммита> [owner/repo]
```

Берёт `docs/{ru,en}/{plugins,plugin-authoring,plugin-platform}.md` текущего клона, убирает строку навигации по репозиторию и переписывает ссылки. Ссылки между этими руководствами и на `command-api.md` становятся путями сайта `/docs/...` с якорями в формате сайта. Неизвестный якорь — ошибка. Остальные относительные ссылки ведут на GitHub в указанный коммит. Результат: `<папка>/<locale>/<статья>.md`. Сайт собирается в FormaCAD-Server.

## Правила документации выпусков

- Заметки о выпуске, [статус](status.md) и [релизы](releases.md) описывают платформу и API, но **не перечисляют и не анонсируют конкретные плагины каталога**, включая примеры авторов проекта. Плагины — работа сообщества; каталог проверяет их сам.
- Скрипты не должны содержать абсолютных путей личных машин, ключей и токенов. Пути передаются параметрами или переменными окружения.

## Проверка

`npm run check:docs` проверяет ссылки этого руководства. `verify-updater-signatures.mjs` проверен на опубликованном установщике 1.2.6, `site-docs.py` — на текущих руководствах.
