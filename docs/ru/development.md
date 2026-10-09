# Разработка
[Документация](index.md) · [English](../en/development.md)

## Зависимости
Node.js 22+, Rust 1.97.1 и [системные зависимости Tauri](https://v2.tauri.app/start/prerequisites/). Windows требует Visual C++ Build Tools и WebView2, macOS — Xcode command-line tools. Пакеты macOS рассчитаны на 11+.

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

## Воспроизводимый Rust
Локальные Rust-проверки и проверенный Windows-пакет используют Rust 1.97.1 и Clippy 0.1.97. [Quality](../../.github/workflows/ci.yml) и [Release](../../.github/workflows/release.yml) закрепляют ту же версию вместо плавающей `stable`; строгий `-D warnings` сохранён. Это согласует окружение проверки, но не заменяет успешный прогон каждой платформы в CI.

После установки rustup выполните из каталога своего клона:
~~~sh
rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
rustup override set 1.97.1
rustc --version
clippy-driver --version
~~~
Override действует для этого клона; глобальную версию других проектов менять не требуется. Ожидаемые версии: `rustc 1.97.1` и `clippy 0.1.97`. Tauri и npm-команды сборки используют выбранный здесь Cargo.

Обновление Rust оформляйте отдельной задачей с владельцем: согласуйте новую фиксированную версию в обоих workflow и этом руководстве, выполните rustfmt, строгий Clippy для всех targets, Rust-тесты с `--locked` и проверки native CAD в подготовленном окружении. Более новый Clippy может вводить дополнительные предупреждения; исправляйте причины, сохраняя строгий режим, и подтверждайте результаты CI перед слиянием.

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
cargo +1.97.1 fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo +1.97.1 clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo +1.97.1 test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
python scripts/test_cad.py
npm run package
~~~

CAD-тестам нужен подготовленный Python. Для геометрических проверок из Rust задайте FORMA_TEST_PYTHON. Установщики собираются на целевой ОС; нужные машины предоставляет [GitHub workflow](../../.github/workflows/release.yml). Подписанные релизы описаны в [руководстве](releases.md).

Браузерные проверки используют реальные пользовательские действия. Общий `e2e/customSelect.ts` дожидается окончания анимаций самого элемента и его контейнеров, выбирает пункт открытого кастомного меню и проверяет итоговый текст закрытого селектора. Не заменяйте такой маршрут `selectOption` для кнопки-комбобокса, принудительным кликом или прямым изменением состояния. Проверка редактора сохраняет требования к высоте 32 px, выравниванию и повторным анимациям; скриншот строки снимается после проверки её полной видимости, без повторной прокрутки локатором. Chromium со SwiftShader не заменяет приёмку установленного WebView.

## Изменения интерфейса
Windows-пакет нативного приложения использует worker, собранный Cargo. Конфигурация native Windows Tauri дополнительно не объявляет промежуточную копию через externalBin: раньше оба маршрута устанавливали одинаковое имя worker, и результат зависел от порядка файлов. Проверяйте, что сгенерированный NSIS содержит ровно одну инструкцию File для worker, затем запускайте smoke именно этого бинарника с упакованными DLL и записывайте его SHA-256. Проверка промежуточного fixture не доказывает состав установщика. Старые версионные установщики сохраняются; локальная проверка не включает установку приложения.

Добавляйте строки в [en.json](../../apps/desktop/src/i18n/en.json) и [ru.json](../../apps/desktop/src/i18n/ru.json), включая подсказки, ошибки и accessibility. Используйте t(), локальное форматирование и цвета темы. Не переводите имена файлов, код, CAD ID и сообщения пользователя/AI. Проверяйте оба языка и обе темы. Скриншоты сохраняются в исключённом docs/verification/.
