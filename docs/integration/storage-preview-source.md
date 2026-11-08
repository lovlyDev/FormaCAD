# Preview исходной папки — точный патч

[English](storage-preview-source.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **применён после разрешения lovlyDev на все семь файлов** 5 ноября 2026.
IPC-основа `5cb38ef` прошла native-проверки; Issue #31 / PR #35.
Исполнитель — wenlixxq, согласование и приёмка — lovlyDev.
Diff: [storage-preview-source.patch](storage-preview-source.patch).

## Контракт и семь файлов

Один host-маршрут preview хранит исходный RepositoryRead. Legacy managed entrypoint
и тестовый worker hook становятся адаптерами к нему. Снимок проверяется до работы,
после worker и после последнего await очистки task map. Metadata-only изменения
по-прежнему отклоняются: preview сохраняет строгую семантику полного снимка.

FCP1 использует task scope исходного открытия в task map и CadTaskGate. Папки с
одинаковым project ID не блокируют и не отменяют друг другу preview; общий CAD
semaphore сохраняется. preview_model и cancel_model_preview получают необязательную
пару sourceId/expectedCommitId; host проверяет настоящее окно, project и commit.
Отказ источника не приводит к managed fallback. Для отмены нужен действительный
source reference; после изменения commit устаревшая ссылка отклоняется.

Imported STEP проверяется через reader. Staging повторно использует уже проверенный
общий model_measurement/staging, переводя только его lifecycle-коды обратно в
существующие preview-коды. Алгоритм worker, packet encoding, лимиты и GLB не меняются.
Прямые Rust build_with_project/build_with_executable сохраняются; один build_staged
обрабатывает legacy и source-bound inputs. Старый verifier без оставшихся callers
удаляется; assertions существующих тестов не ослабляются.

Rust относительно `apps/desktop/src-tauri/src/`:

- `model_preview/host.rs`
- `model_preview/host/tests.rs` — общий reader в существующем cleanup-await тесте.
- `model_preview/build.rs`
- `model_preview/imports.rs`
- `model_preview/mod.rs`
- новый `model_preview/host/folder_tests.rs`

Frontend: `apps/desktop/src/features/model-preview/previewApi.ts` — необязательный
FolderSourceRef в preview/cancel, сохранение packet/hash/GLB проверок и старых вызовов.
Повторно используется существующая переведённая ошибка PROJECT_FOLDER_UNSUPPORTED.

## Проверки и границы

Применимость, staged rustfmt и полный TypeScript с виртуальной подстановкой
preview API прошли. После применения прошли 294 frontend-теста / 104 файла и
production build. На `4b9615843b6cf72eeaecf62f28fc76f48d904ad5` прошли
[оба строгих Clippy/comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/42050145205)
и [native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/42050145085).
Скачанный job `127323748900` подтверждает все три preview-теста по имени и
отдельный запуск ровно 3 PASS. Полный пакет: 540 library PASS / 18 ignored,
11 smoke PASS, 41 host PASS, два measurement PASS и три roundtrip PASS.
Собственный workflow требует три выполненных preview-теста, исключая пустой фильтр.
Три теста создают настоящий FCP1 STEP 40×20×10 и повреждённый managed-двойник с тем же ID;
проверяют volume 8000, bounds, program SHA, GLB, task group, отказ после worker close
и после close во время cleanup mutex await, SQL/app purity, освобождение задач
и отсутствие оставшихся временных входов в `.transient/previews`.
Дополнительная регрессия в том же разрешённом новом тестовом файле открывает две
независимые папки с одинаковыми project IDs, ставит обе настоящие preview-задачи
в очередь, отменяет только первую и проверяет геометрию второго результата.
Production diff и семь согласованных путей при добавлении этой проверки не менялись.
Все три описанных preview-сценария выполнены в указанном native CI. Этот результат
не заменяет будущую проверку workspace/source state и GUI.
[Quality указанного SHA](https://github.com/lovlyDev/FormaCAD/actions/runs/42050157179)
также завершился успешно.

Патч не подключает workspace source state или кнопку папки и не завершает этап 4.
Нет изменений native/Cargo/C++/protocol/permissions/SQL/версий.
Для общих preview/frontend файлов §5 ТЗ требует согласования до применения.

Порядок запусков собственных storage workflows для артефактов согласования:
[CI и review artifacts](storage-ci-review-artifacts.md).
