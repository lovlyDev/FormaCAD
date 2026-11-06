# Ручное сечение исходной папки — точный патч

[English](storage-manual-section-source.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **применён после явного согласования всех шести файлов; native CI ожидается**. Issue #31 / PR #35, исполнитель wenlixxq,
согласование и приёмка lovlyDev. Основа runtime `64bde10`.
Diff: [storage-manual-section-source.patch](storage-manual-section-source.patch).

## Контракт и шесть файлов

Ручное сечение сохранённого STEP получает RepositoryRead исходного открытия.
Authored CAD IR не требуется. До/после worker сохраняются строгие проверки всего
снимка, current revision и sealed STEP. В отличие от точных authored-измерений,
metadata-only изменения по-прежнему инвалидируют этот запрос, как в старом пути.
Группа CAD-задачи берётся из исходного reader, а worker и его алгоритм не меняются.

Проверенные байты единственного STEP перемещаются в существующий stage_assets_with.
Нет чтения AppData вместо папки и дополнительной полной копии staging-буфера.
Существующий section_model принимает необязательную пару sourceId/expectedCommitId;
неполная пара отклоняется, host проверяет окно и commit, ошибки папки не дают fallback.
Старые managed-вызовы и тестовые hooks сохраняются. SourceRef не сохраняется в БД.

Rust относительно `apps/desktop/src-tauri/src/`:

- `model_section/capture.rs`
- `model_section/build.rs`
- `model_section/build_tests.rs` — только pub(super) существующего fixture-модуля.
- `model_section/mod.rs`
- новый `model_section/folder_build_tests.rs`

Frontend: `apps/desktop/src/features/viewer/sections/sectionApi.ts`.
API принимает необязательный FolderSourceRef, сохраняя проверку source SHA.
Для browser-папки используется существующий перевод PROJECT_FOLDER_UNSUPPORTED.

## Проверки и незавершённые условия

Применимость и staged rustfmt прошли. Полная проверка TypeScript с виртуальной
подстановкой всех пяти подготовленных CAD API прошла; это была проверка подготовленного патча. После согласованного применения production build прошёл в рабочей ветке.
Новая native-регрессия подготовлена, но **ещё не выполнялась**: настоящий STEP
40×20×10 помещается в FCP1 без authored program/preview, managed-двойник повреждается.
Проверяются source hash, периметр сечения 120 мм, отказ после закрытия во время
настоящего worker, неизменность SQL/app state и очистка transient workspace.
После применения нужны strict/native CI, frontend tests/build и явный PASS этого теста.

Патч ещё не подключает workspace state или кнопку папки и не завершает этап 4.
Native/Cargo/C++/protocol/permissions/SQL/версии неизменны. Общие model_section и
frontend-файлы согласованы владельцем по §5 ТЗ перед применением.
