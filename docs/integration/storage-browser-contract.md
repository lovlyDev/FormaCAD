# Browser catalog contract regressions / Browser-регрессии контракта каталога

Base: `1ebfccde1b36f819657fbe4788f6873213b87723`. [Exact patch](storage-browser-contract.patch). NOT applied. Five existing e2e files, 23 insertions/10 deletions. git apply --check passes; tests after patch are not run before approval.

## RU

Quality 41220743517: 227 unit PASS; browser35PASS/35skipped/10FAIL. Все четыре платформенных Rust jobs, CAD, native consumers41220743516 и storage comparison41220743451 успешны. Общий Quality не зелёный: frontend и существующий Wasmtime security audit.

Восемь browser failures вызваны устаревшим mock list_projects=[]: actual Dashboard ожидает CatalogReport.entries, падение React мешает открыть Settings. Два fixtures общего model-apply/reference harness тоже должны использовать новый wire contract и read_project_snapshot; это подготовка их native сценариев без изменения worker/геометрических проверок.

Workspace preview test заменяет ожидание автоматического img на default SVG и сравнение точного localStorage payload до/после просмотра каталога. Явные pin/rename/delete проверки сохраняются. Pin-animation локально воспроизведён на неизменном тесте: снимок ошибки показывает workspace First card вместо Dashboard. goto('/') конфликтует с сохранённым route restore. Оба сценария возвращаются через настоящую button.brand, которая вызывает текущую навигацию/refresh; таймауты, skips и продуктовый route restore не меняются.

Точные файлы:
1. apps/desktop/e2e/kernel-health.spec.ts
2. apps/desktop/e2e/settings-health-demand.spec.ts
3. apps/desktop/e2e/modelApplyHarness.tsx
4. apps/desktop/e2e/referenceMeasurementHarness.tsx
5. apps/desktop/e2e/workspace.spec.ts

Production App/backend/native/Cargo/protocol/версии не меняются. Это общие UI/native test fixtures; по §5 запрашивается согласование перед применением. После него выполнить все 10 ранее падавших тестов, затем соответствующий полный browser-прогон. Native geometry fixtures не объявлять проверенными без реального worker fixture.

## EN

This test-only patch updates mocked list_projects to CatalogReport and adds the existing pure target-read command to model-apply fixtures. The reference harness consumes ready entries explicitly instead of assuming an array. Kernel/settings assertions remain unchanged.

The card test verifies default artwork and byte-identical browser project storage before explicit pin/rename/delete, replacing the removed automatic thumbnail behavior. The pin-animation failure was reproduced on unchanged code: the saved project route reopened the workspace. Tests return home through the real brand button rather than racing root navigation against route restoration. No timeouts/skips are changed. Apply-check passed; patched execution still awaits shared-fixture approval. No production/native/Cargo/protocol/version edits.