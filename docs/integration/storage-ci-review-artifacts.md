# Storage CI и артефакты согласования

[English](storage-ci-review-artifacts.en.md) · [Preview](storage-preview-source.md)

Два собственных workflow ветки `codex/storage-week` — Storage comparison и Storage
native consumers — исключают из push-триггера только `docs/integration/**`.
Коммит исключительно с контрактами/неприменёнными patch-артефактами не запускает
новую CAD-сборку и не отменяет текущий comparison. При смешанном коммите любой
путь вне этого каталога запускает полный прежний workflow.

Это не исключает `docs/fixtures`, исходники frontend/Rust/C++, Cargo, build scripts
или конфигурации CI. Manual workflow_dispatch сохранён. Jobs, команды, test filters,
PASS guards, permissions и concurrency не меняются. Общий Quality workflow владельца
не изменён; проверки документации/i18n по-прежнему обязательны.

При введении фильтра сравнением с `4b96158` проверены YAML обоих файлов и
структурная идентичность определений после удаления
единственного нового поля paths-ignore. Проверены случаи docs-only, docs+Rust,
frontend, fixture, C++, Cargo.lock и самого workflow. Поиск прямых runtime-ссылок на
docs/integration в apps/cad-core/scripts/workflows не дал совпадений; общий валидатор
документации сохранён.
Этот фильтр не является доказательством правильности будущих source-изменений:
каждый runtime-коммит требует собственных проверок.
