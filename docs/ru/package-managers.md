# Установка через winget

[Документация](index.md) · [English](../en/package-managers.md) · [Релизы](releases.md)

На Windows FormaCAD можно поставить из каталога Microsoft winget. winget скачивает тот же установщик из GitHub Releases и сверяет его SHA-256.

## Действия пользователя

1. Открыть «Терминал» или PowerShell.
2. Установить: `winget install lovlyDev.FormaCAD`
3. Обновить: приложение обновляется само; вручную — `winget upgrade lovlyDev.FormaCAD`.
4. Удалить: `winget uninstall lovlyDev.FormaCAD` или «Параметры → Приложения».

winget ставит FormaCAD для текущего пользователя, права администратора не нужны. Первая версия 1.6.1 [отправлена в каталог](https://github.com/microsoft/winget-pkgs/pull/470914) и прошла автоматическую проверку; команда заработает, когда пакет примет модератор Microsoft.

## Технический маршрут

- Описание пакета лежит в `packaging/winget/manifests/l/lovlyDev/FormaCAD/<версия>/`: версия, установщик (NSIS для текущего пользователя, `ProductCode` `Forma`) и описания на английском и русском. Проверка: `winget validate --manifest <папка>`.
- Новые версии отправляет задача `package-managers` в Release workflow после публикации стабильного релиза: `wingetcreate update lovlyDev.FormaCAD --version <X.Y.Z> --urls <EXE> --submit`. Она создаёт PR в `microsoft/winget-pkgs` от имени владельца токена.
- Нужен секрет репозитория `WINGET_TOKEN`: классический токен GitHub с правом `public_repo`. Без секрета шаг пропускается с предупреждением, релиз не падает.

### Как создать и добавить `WINGET_TOKEN`

1. GitHub → аватар справа вверху → **Settings** → внизу слева **Developer settings** → **Personal access tokens** → **Tokens (classic)** → **Generate new token** → **Generate new token (classic)**.
2. **Note**: `FormaCAD winget`. **Expiration**: `No expiration` или год (тогда продлевать).
3. В списке прав поставить одну галочку **public_repo** (внутри раздела `repo`). Остальное не отмечать.
4. **Generate token** внизу и сразу скопировать токен `ghp_…`: GitHub показывает его один раз.
5. Репозиторий `lovlyDev/FormaCAD` → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**.
6. **Name**: `WINGET_TOKEN`, **Secret**: вставить токен → **Add secret**.

Токен не пересылать в чат, не сохранять в файлах репозитория.

## Ограничения

- Беты (`-beta.N`) в winget не попадают.
- Каждую новую версию тоже проверяют модераторы Microsoft, поэтому в winget она появляется с задержкой от нескольких часов до пары дней.
- Другие менеджеры пакетов (Homebrew, Flathub) не поддерживаются: на macOS и Linux ставить DMG, AppImage, DEB или RPM из релиза.
