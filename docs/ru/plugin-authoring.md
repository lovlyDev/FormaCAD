# Руководство автора плагина

[Документация](index.md) · [English](../en/plugin-authoring.md) · [Спецификация](plugin-platform.md) · [Плагины для пользователя](plugins.md)

Плагин FormaCAD — WebAssembly-компонент с контрактом plugin-api 0.2 — [`formacad:plugin`](../plugin-api/wit/formacad-plugin.wit), упакованный в `.cadplugin`. Полный рабочий пример — [`plugins/examples/simple-gear`](../../plugins/examples/simple-gear/src/lib.rs): шестерня из обычных операций CAD IR.

## Структура проекта

```text
my-plugin/
  Cargo.toml          crate-type = ["cdylib"], зависимость wit-bindgen
  manifest.json       по схеме docs/schemas/plugin-manifest.schema.json
  wit/world.wit       ваш world: include host + нужные экспорты
  src/lib.rs          реализация экспортов
  locales/ru.json     строки для %key% в манифесте и для самого плагина
  locales/en.json
  assets/…            необязательные данные (читаются через assets.read)
```

`wit/world.wit` подключает весь хост и экспортирует только то, что вы добавляете:

```wit
package example:my-plugin;

world my-plugin {
    include formacad:plugin/host@0.1.0;
    export formacad:plugin/feature@0.1.0;
}
```

В `src/lib.rs`:

```rust
wit_bindgen::generate!({
    path: ["<путь к docs/plugin-api/wit>", "wit"],
    world: "example:my-plugin/my-plugin",
    generate_all,
});
```

Положите копию `formacad-plugin.wit` в свой репозиторий (например `wit/deps/formacad-plugin/`) и укажите этот путь. Если копия лежит в `wit/deps/`, достаточно `path: "wit"`: wit-bindgen сам подключает `deps`, а повторное указание папки даёт ошибку «re-add package». В компонент попадут только те интерфейсы хоста, которые код реально вызывает, — каждый из них должен быть покрыт правом в `manifest.json` (таблица в [спецификации, раздел 11](plugin-platform.md#11-интерфейс-хоста-formacadplugin-plugin-api-02)).

## Что возвращает плагин

- `feature.build` и `importer.import-file` возвращают JSON-массив команд [Command API](command-api.md): обычно `add_feature` с операциями CAD IR v2 (`circle`, `rectangle`, `sketch2d`, `extrude`, `revolve`, `rotate`, `translate`, `boolean`, `hole`, …) и `add_body`.
- `command.run` возвращает `{"message":"…","commands":[…]}`; пустой массив — «ничего не менять». Команда без права `model.write` может вернуть только пустой массив.
- `exporter.export-file` возвращает байты файла.
- `panel.render` возвращает `{"blocks":[{"type":"text","text":"…"},{"type":"form","fields":[…],"submit":{"command":"<id>","label":"…"}}]}`.
- Все новые ID начинаются с `context.idPrefix`. Ошибка — `plugin-error { code, message }`; пользователь увидит её как ошибку плагина.

Хост проверяет параметры формы до вызова (тип, min/max, варианты), а результат — строгими типами CAD IR, валидацией документа и построением BREP. Любая ошибка оставляет модель без изменений.

## Сборка и упаковка

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cargo build --release --target wasm32-unknown-unknown
```

```bash
cargo run --release --manifest-path <FormaCAD>/plugins/tools/cadplugin/Cargo.toml -- pack --manifest manifest.json --wasm target/wasm32-unknown-unknown/release/my_plugin.wasm --dir . --out dist
```

`cadplugin pack` превращает core-модуль от wit-bindgen в компонент, добавляет `manifest.json`, `locales/`, `assets/` и иконку и пишет `dist/<id>-<version>.cadplugin` с фиксированными датами (одинаковый вход → одинаковый SHA-256). Пакет ограничен 20 МБ сжатого и 60 МБ распакованного размера, 200 файлами и глубиной путей 6.

## Проверка у себя

Установите `dist/*.cadplugin` через «Настройки → Плагины → Установить из файла…»: FormaCAD выполнит те же проверки, что и каталог (схема манифеста, импорты против прав, экспорты против `contributes`). Вызовите операцию из палитры команд (`Ctrl K`). Журнал вызовов — в карточке плагина; строки `log.write` попадают туда же.

Лимиты вызова: топливо, память 256 МБ, 10 с. Делайте вычисления детерминированными и не храните состояние между вызовами в памяти — каждый вызов получает новый экземпляр; для данных используйте `storage`.

## Релиз для каталога

1. Тег `vX.Y.Z` совпадает с `version` в манифесте; `id` после первой публикации не меняется.
2. Прикрепите к GitHub-релизу `<id>-<version>.cadplugin`.
3. Один раз подключите репозиторий: войдите на `plugins.formacad.ru` аккаунтом FormaCAD; привяжите GitHub к аккаунту на formacad.ru («Аккаунт → Безопасность»), если он ещё не привязан; в «Кабинете автора» нажмите «Установить GitHub App», поставьте [FormaCAD Plugins](https://github.com/apps/formacad-plugins) на репозиторий (нужны только чтение содержимого и метаданных) и выберите «Публиковать отсюда».
4. Дальше каждый опубликованный (не черновик и не пререлиз) GitHub-релиз проверяется автоматически: ассет, ZIP, манифест (`repository` = этот репозиторий, `author.github` = ваш GitHub), импорты WASM против прав, скриншоты. Прошедшая версия появляется в каталоге сразу; причина отказа видна в кабинете, «Проверить снова» повторяет проверку. Каталог хранит свою копию пакета и подписывает запись индекса.
5. Новые права в новой версии пользователи подтверждают заново — объявляйте только то, что действительно нужно.

## Пример: simple-gear

Пример строит диск, прямоугольный зуб, поворачивает его N раз, объединяет и сверлит отверстие. Права: `model.write` (вклад `features`) и `notifications` (сообщение после построения). Готовый пакет для тестов: [`plugins/examples/simple-gear/prebuilt`](../../plugins/examples/simple-gear/prebuilt). Тест [`plugin_gear.rs`](../../apps/desktop/src-tauri/tests/plugin_gear.rs) сверяет объём построенной OCCT-шестерни с аналитической формулой.

## Пример: запись видео (plugin-api 0.2)

Команда без `model.write` может попросить FormaCAD записать видео вращения модели. Объявите право `viewport.capture`, `"api": "0.2"` и `"minAppVersion": "1.5.0"`, проверьте форму и вызовите хост:

```rust
use formacad::plugin::viewport;
viewport::record_orbit(viewport::Orbit {
    width: 1920, height: 1080, fps: 30, duration_ms: 8000, turns: 1.0, clockwise: false,
    up: viewport::UpAxis::Z, tilt_deg: 0.0, elevation_deg: 20.0, start_deg: 0.0, zoom: 1.0,
    background: viewport::Background::Theme, format: viewport::VideoFormat::Mp4, bitrate_kbps: 0,
})?;
// затем верните {"message":"…","commands":[]}
```

Запись начинается после завершения вызова: CAD спрашивает путь, рендерит кадры и кодирует видео сам. Готовый плагин с формой и переводами: [lovlyDev/formacad-orbit-video](https://github.com/lovlyDev/formacad-orbit-video). Диапазоны значений — в [спецификации, раздел 12](plugin-platform.md#12-уточнения-реализации).

## Кнопка в панели 3D-вида

Операция или команда с полем `icon` из списка `box`, `camera`, `clapperboard`, `cog`, `compass`, `film`, `grid-3x3`, `layers`, `monitor-play`, `package`, `pen-tool`, `printer`, `rotate-3d`, `ruler`, `scan`, `scissors`, `sparkles`, `square-dashed`, `video`, `wand-2`, `wrench` получает в FormaCAD 1.5.1 и новее свою кнопку в панели инструментов 3D-вида; подсказка — «<плагин>: <название>». С другим значением или без `icon` действие остаётся в палитре команд и в меню «Действия плагинов».
