# Preview исходной папки — точный патч

[English](storage-preview-source.en.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **подготовлен, не применён**. Основа `5cb38ef`, Issue #31 / PR #35.
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
preview API прошли. Патч не скомпилирован; новые native-тесты **ещё не выполнены**.
Они создают настоящий FCP1 STEP 40×20×10 и повреждённый managed-двойник с тем же ID;
проверяют volume 8000, bounds, program SHA, GLB, task group, отказ после worker close
и после close во время cleanup mutex await, SQL/app purity и освобождение задач.
После применения нужны оба строгих Clippy, существующие managed/native тесты,
два явно выполненных новых теста и frontend tests/build.

Патч не подключает workspace source state или кнопку папки и не завершает этап 4.
Нет изменений native/Cargo/C++/protocol/permissions/SQL/версий.
Для общих preview/frontend файлов §5 ТЗ требует согласования до применения.
