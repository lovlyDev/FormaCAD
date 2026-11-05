# Preview редактора и AI-review: исходное открытие

[English](storage-preview-renderer-source.en.md) · [Основной патч состояния](storage-workspace-source-state.md) · [Host API](../ru/storage-folder-host-api.md)

Статус: **подготовлен, не применён; отдельное согласование ещё не запрошено**.
Issue #31 / PR #35, исполнитель wenlixxq, приёмка lovlyDev.
Точный [патч](storage-preview-renderer-source.patch) применяется **после**
`storage-workspace-source-state.patch`, включая исправление ключа черновика.
Это зависимый объём; разрешение на backend preview не является разрешением на него.

## Поведение

`useModelPreview` получает исходный binding открытия. Папочные build/cancel
передают существующему IPC именно его FolderSourceRef. Явно отсутствующий или
чужой binding не превращается в managed-вызов. При смене открытия старая геометрия
сразу скрывается, исходная задача отменяется в собственной группе, поздний объект
освобождается. Ошибка старой задачи не отображается в новом открытии. Смена commit
внутри открытия заново запускает строгий preview; прежняя геометрия не удерживается.
Смена текста внутри того же источника сохраняет прежнее поведение debounce и
последовательной очереди с предыдущей геометрией до готовности новой.

ProgramDialog передаёт binding и хранит папочный черновик в памяти через общий
source preference. Ключ ревизии входит в семейство черновика; поздний setter не
меняет новый ключ и не пишет managed preferences. Глобальные настройки редактора
продолжают использовать прежний маршрут.

App сохраняет binding AI-review до await в диалоге подтверждения; уже заданный
binding не заменяется текущим. CandidateReview сравнивает исходное открытие с
активным дополнительно к проверке проекта/ревизии. Просмотр кандидата и отдельная
копия сохранённой GLB используют этот же источник. При подмене папки с одинаковыми
project/revision IDs принятие блокируется. Baseline loader скрывает старую сцену
и освобождает поздние объекты, а host-чтение и проверку после декодирования выполняет
ранее подготовленный modelLoader.

## Все 8 путей

Относительно `apps/desktop/src/`:

1. `features/model-preview/useModelPreview.ts`
2. новый `features/project-sources/previewRequests.test.tsx`
3. `app/dialogs/ProgramDialog.tsx`
4. `features/agents/review/CandidateReview.tsx`
5. `features/agents/review/useSavedBaseline.ts`
6. `app/App.tsx`
7. новый `features/project-sources/baselineSource.test.tsx`
8. новый `features/project-sources/candidateSource.test.tsx`

## Проверки и пределы

В полном изолированном снимке с обоими патчами: **347 тестов / 115 файлов PASS**,
production build PASS, i18n 1483 PASS. Этот патч добавляет 13 тестов; прежние тесты
очереди preview и AI-review сохранены. Основной патч отдельно прошёл 334 теста /
112 файлов и сборку. Применимость зависимого diff проверена на изолированной основе.
Это renderer fixtures/mocks, а не доказательство геометрии worker или установленного
приложения. Native preview имеет отдельные уже пройденные STEP/FCP1 проверки.

Rust/native/Cargo/protocol/backend IPC/SQL/версии не меняются; новых пользовательских
строк и видимой кнопки папки нет. Не подключены остальные measurement/history/
import/export consumers, folder writer и permission UI. Нельзя объявлять этап 4
завершённым или доступный пользовательский маршрут готовым по этим двум патчам.
По §5 ТЗ общие frontend/App файлы требуют согласования до применения.
