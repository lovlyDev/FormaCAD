# Complete starter prompt for your friend's AI

[Collaboration](collaboration.md) · [Documentation](index.md) · [Русский](../ru/collaboration-ai-prompt.md)

Each person uses a separate FormaCAD clone and AI session. First confirm whose turn it is, claim an Issue and appoint the release owner. Your friend then pastes the prompt below, replacing the angle-bracket fields. Repository reading and local development do not require access to the other person's private chat.

The friend's account is **wenlixxq** and the repository owner is **lovlyDev**. Before the first push, verify that `wenlixxq` has received and accepted an invitation with the required access; this document does not confirm that it was granted. The prompt assumes the friend's confirmed turn; change the implementer for the return turn; project head and acceptance owner remain lovlyDev.

The prompt authorizes pushing the **assigned task branch** after checks, creating a PR and updating its Issue. Project head lovlyDev reviews the result and decides whether to merge; no second reviewer is required; tags, releases and installation have separate authority. The current release owner's authorization does not grant every future session permanent publication rights. Do not include secrets or user models.

```text
You are working on FormaCAD, a local AI-native CAD platform, in my separate clone.
Repository: https://github.com/lovlyDev/FormaCAD
My confirmed active turn: wenlixxq (lovlyDev must confirm the handoff).
Other human: lovlyDev; they are currently reading/reviewing only, not editing.
Assigned Issue: #<number>.
Task branch: task/<number>-wenlixxq-<short-name>.
Handed-over main base SHA: <SHA>.
Owned files/modules: <explicit paths>.
Other owners and paths: <names and paths>.
Agreed API and dependencies: <contract / related Issues>.
This turn's goal and acceptance: <observable behavior and checks>.
Project head and acceptance owner: lovlyDev (on both turns).
Version/build/release integrator: <name / login>.
Release-publication authority this turn: <not authorized / explicit separate instruction>.

1. Read AGENTS.md, README.md, docs/architecture-2.0.md, docs/en/status.md, docs/en/collaboration.md,
docs/en/project-snapshots.md, docs/en/architecture.md, docs/en/development.md,
docs/en/roadmap.md and the assigned Issue/latest PR handoff. Inspect actual affected
source modules. Record the branch and SHA. If Issue access is unavailable, ask the
owner for its sanitized body; continue independent readonly source inspection, but
do not invent task boundaries or API contracts.

2. Do not assume another PC's private files exist here. .local handoffs, private build
logs and another chat's memory are not transferred by GitHub. The shared full plan
is docs/architecture-2.0.md and travels with source; its 2.0 boundary, later 2.x stages
and acceptance gate are binding. If that file or a specification clarification is
unavailable, request the owner's confirmed text before dependent decisions. A short
roadmap does not replace the master plan. Do not claim another agent's tests were
run or that missing features are implemented.

3. The overall goal is the agreed complete Forma 2.0 platform, not changing version
numbers. It includes offline local CAD without required Python, isolated C++/OCCT
worker, typed Command API/CAD IR for all changes, checked atomic AI plans and undo/
redo, named bodies/parameters/features, CAD face/edge selection with honest reference
limits, exact measurements/sections, comparison, STEP/mesh exchange, Codex/Claude/
Custom CLI, the agreed usable UI, RU/EN and both themes. Portable project folders
must become genuine project authority; the application index stores recent projects
and settings. All consumers require shared backend/location/session/epoch binding,
OS leases, readonly mode and checked copies, explicit recovery, backups/migrations,
safe media loss, full/redacted cadpack and reliable transfer. Select the storage
format using actual performance/recovery evidence, not a database name. Genuine
legacy 1.1 projects produce a NEW project from the last saved STEP; do not promise
recovered authored operations/history or silently change old data. Windows comes
first, but full 2.0 acceptance requires macOS arm64/x64 and Linux, bundled CAD,
offline installed-app scenarios, crashes/cancel/corruption/two-process access and
export checks. WebGPU requires functional WebGL2 fallback. Full sketching/assemblies/
drawings/Lua/WASM/manufacturing/simulation are separate 2.x stages; do not claim they
already ship or expose empty promise panels. This summary neither overrides the
approved original specification nor establishes 2.0 readiness.

4. We take turns. Do not edit before my turn and Issue ownership are confirmed.
Check git status --short BEFORE fetch/pull or switching branches. Preserve all local
files and unfamiliar changes. With a clean clone, fetch origin/main, update main
with pull --ff-only, compare SHA to the handoff, then create the task branch. On
divergence stop dependent edits and explain the conflict; do not use reset --hard,
git clean, force push, rewriting another person's history or Git GC to erase work.
For an explicitly handed-over unfinished PR branch, continue that branch rather
than claiming it has already merged into main.

5. Complete the assigned task within my modules. Each independent capability has a
clear module/file. Coordinate shared i18n, lockfiles, migrations, IPC and versions
with their owners. Applied migrations are immutable; new ones use LF. Immediately
update RU/EN text and topical docs, preserve both themes, scene state and existing
data compatibility. Do not replace real selection/worker tests with setters,
weakened assertions or a UUID-only proof. If using subagents, assign nonoverlapping
files and review their work; this does not give the second PC a turn. Coordinate
heavy native builds instead of launching competing compiler actors.

6. Run appropriate checks and project requirements. Before handoff verify check:docs,
check:i18n, check:migrations and check:version; choose frontend/Rust/C++ and genuine
browser/native scenarios for the affected route. Check error/cancel/stale/recovery
and data behavior when affected. Report actual commands/results, skipped checks
and limits. Do not label mocked browser testing as installed WebView, local SSD as
NAS acceptance, pure reads as external-folder authority or source build as ready EXE.

7. After checks, I authorize small commits of assigned files, pushing this task
branch to origin, creating its Issue-linked PR and updating its Issue with evidence.
Inspect diffs before git add and stage explicit paths. Exclude .local, private
handoffs, user databases/CAD files, tokens, keys and build caches. Do not push directly
to main. Project head lovlyDev reviews the result and decides whether to merge; do not require another reviewer. Naming the acceptance owner does not establish completed review or authorize merging. Do not tag, publish
a release or install without a separate explicit instruction. If appointed this
turn's integrator, synchronize npm/workspace/lockfile/Cargo/Tauri versions after
acceptance, build a NEW Windows EXE and check its existence, version, hash/contents
and relevant scenarios. Preserve previous releases under AGENTS.md. Publishing
that checked release requires the separate authority specified above.

8. Report Issue, branch, commit SHA, files, API, actual checks, limits and PR link;
include version/new EXE link if you are the appointed integrator. State what this
task actually completed and what remains before full 2.0 acceptance. Prepare the
human handoff: merged main SHA (or explicitly unmerged PR branch), next Issue/scope/
owner and release owner. Humans confirm the turn transfer; after confirmation stop
editing until the next assigned turn.
```

Connection and pull commands are in the [main guide](collaboration.md); the complete shared specification is the [architecture plan](../architecture-2.0.md). The prompt does not automatically assign a human, supply signing secrets, establish installation or certify another PC's tests. Read the current [status](status.md) instead of relying on an old chat's version number.
