# Руководство автора плагина

[Документация](index.md) · [English](../en/plugin-authoring.md) · [Спецификация](plugin-platform.md) · [Плагины для пользователя](plugins.md)

Плагин FormaCAD — WebAssembly-компонент с контрактом [`formacad:plugin@0.1`](../plugin-api/wit/formacad-plugin.wit), упакованный в `.cadplugin`. Полный рабочий пример — [`plugins/examples/simple-gear`](../../plugins/examples/simple-gear/src/lib.rs): шестерня из обычных операций CAD IR.

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

Положите копию `formacad-plugin.wit` в свой репозиторий (например `wit/deps/formacad-plugin/`) и укажите этот путь. В компонент попадут только те интерфейсы хоста, которые код реально вызывает, — каждый из них должен быть покрыт правом в `manifest.json` (таблица в [спецификации, раздел 11](plugin-platform.md#11-интерфейс-хоста-formacadplugin01)).

## Что возвращает плагин

- `feature.build` и `importer.import-file` возвращают JSON-массив команд [Command API](command-api.md): обычно `add_feature` с операциями CAD IR v2 (`circle`, `rectangle`, `sketch2d`, `extrude`, `revolve`, `rotate`, `translate`, `boolean`, `hole`, …) и `add_body`.
- `command.run` возвращает `{"message":"…","commands":[…]}`; пустой массив — «ничего не менять».
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
3. Установите GitHub App «FormaCAD Plugins» на репозиторий и опубликуйте плагин на `plugins.formacad.ru` (раздел 5 спецификации). Каталог хранит свою копию пакета и подписывает запись индекса.
4. Новые права в новой версии пользователи подтверждают заново — объявляйте только то, что действительно нужно.

## Пример: simple-gear

Пример строит диск, прямоугольный зуб, поворачивает его N раз, объединяет и сверлит отверстие. Права: `model.write` (вклад `features`) и `notifications` (сообщение после построения). Готовый пакет для тестов: [`plugins/examples/simple-gear/prebuilt`](../../plugins/examples/simple-gear/prebuilt). Тест [`plugin_gear.rs`](../../apps/desktop/src-tauri/tests/plugin_gear.rs) сверяет объём построенной OCCT-шестерни с аналитической формулой.
