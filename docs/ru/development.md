# Разработка
[Документация](index.md) · [English](../en/development.md)

## Зависимости
Node.js 22+, Rust stable и [системные зависимости Tauri](https://v2.tauri.app/start/prerequisites/). Windows требует Visual C++ Build Tools и WebView2, macOS — Xcode command-line tools. Пакеты macOS рассчитаны на 11+.

Ubuntu 22.04+:
~~~sh
sudo apt-get update
sudo apt-get install -y build-essential curl wget file libssl-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2 rpm
npm ci
npm run desktop
~~~

При нескольких установках Visual Studio:
~~~powershell
. ./scripts/windows-env.ps1
npm run desktop
~~~
[Скрипт](../../scripts/windows-env.ps1) выбирает полный набор C++-заголовков и библиотек.

## CAD-окружение
Используйте Python 3.12 и [requirements-cad.txt](../../requirements-cad.txt):
~~~sh
python -m venv .venv
# Windows
.venv/Scripts/python -m pip install -r requirements-cad.txt
# macOS / Linux
.venv/bin/python -m pip install -r requirements-cad.txt
~~~
Выберите интерпретатор в «Настройки → CAD-окружение». Путь сохраняется при обновлениях. FORMA_PYTHON — необязательный резервный вариант. CLI нужно установить и авторизовать отдельно. Запуск с рабочего стола также ищет инструменты в стандартных каталогах Homebrew и локальных программ без выполнения shell-профилей.

## Проверки
~~~sh
npm run check:version
npm run check:i18n
npm run check:docs
npm run lint
npm test
npx playwright install chromium
npm run test:e2e
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
python scripts/test_cad.py
npm run package
~~~

CAD-тестам нужен подготовленный Python. Для геометрических проверок из Rust задайте FORMA_TEST_PYTHON. Установщики собираются на целевой ОС; нужные машины предоставляет [GitHub workflow](../../.github/workflows/package.yml). Подписанные релизы описаны в [руководстве](releases.md).

## Изменения интерфейса
Добавляйте строки в [en.json](../../apps/desktop/src/i18n/en.json) и [ru.json](../../apps/desktop/src/i18n/ru.json), включая подсказки, ошибки и accessibility. Используйте t(), локальное форматирование и цвета темы. Не переводите имена файлов, код, CAD ID и сообщения пользователя/AI. Проверяйте оба языка и обе темы. Скриншоты сохраняются в исключённом docs/verification/.
