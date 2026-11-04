# Установка через менеджеры пакетов

[Документация](index.md) · [English](../en/package-managers.md) · [Релизы](releases.md)

FormaCAD можно ставить не только с GitHub Releases, но и через менеджеры пакетов. Они скачивают те же установщики из GitHub Releases и проверяют SHA-256.

## Команды

| Система | Команда | Состояние |
|---|---|---|
| Windows | `winget install lovlyDev.FormaCAD` | Пакет 1.6.1 отправлен в каталог Microsoft ([PR](https://github.com/microsoft/winget-pkgs/pull/470914)), работает после принятия |
| macOS | `brew install --cask lovlydev/formacad/formacad` | Собственный каталог `lovlyDev/homebrew-formacad`, работает после его создания |
| Linux | — | Flathub запланирован; пока AppImage, DEB и RPM из релиза |

Обновления: приложение обновляется само. Ещё можно выполнить `winget upgrade lovlyDev.FormaCAD` или `brew upgrade --cask formacad`.

## winget

- Описание пакета лежит в `packaging/winget/manifests/l/lovlyDev/FormaCAD/<версия>/`: версия, установщик (NSIS для текущего пользователя, `ProductCode` `Forma`) и описания на английском и русском. Проверка: `winget validate --manifest <папка>`.
- Первую версию принимают модераторы Microsoft после автоматической проверки в песочнице. При первом PR бот попросит подписать CLA: ответить в PR комментарием `@microsoft-github-policy-service agree`.
- Новые версии отправляет задача `package-managers` в Release workflow: `wingetcreate update lovlyDev.FormaCAD --version <X.Y.Z> --urls <EXE> --submit`. Нужен секрет `WINGET_TOKEN`: классический токен GitHub с правом `public_repo`. Без секрета шаг пропускается с предупреждением.

## Homebrew

- В основной каталог Homebrew (`homebrew/cask`) приложение пока не попадёт: нужны подписанное и нотаризованное приложение и известность репозитория. Поэтому используется свой каталог (tap) `lovlyDev/homebrew-formacad`.
- Описание пакета `Casks/formacad.rb` генерирует `node scripts/homebrew-cask.mjs <версия> <SHA256SUMS>`; копия для 1.6.1 лежит в `packaging/homebrew/`. Один пакет для Apple Silicon и Intel, DMG из релиза.
- Приложение пока не нотаризовано, поэтому после установки пакет снимает с `Forma.app` отметку карантина. Иначе Gatekeeper блокирует первый запуск.
- `brew uninstall --zap --cask formacad` удаляет приложение и его данные из `~/Library` (`studio.forma.cad`).
- Новые версии обновляет задача `package-managers`: скачивает `SHA256SUMS` релиза и записывает новый `Casks/formacad.rb` в каталог. Нужен секрет `HOMEBREW_TAP_TOKEN`: fine-grained токен только на `lovlyDev/homebrew-formacad` с правом Contents: Read and write.

## Ограничения

- Беты (`-beta.N`) в менеджеры пакетов не попадают.
- Flathub требует сборки из исходников в своей системе сборки, включая OpenCascade. Это отдельная задача.
