# Two-PC collaboration with AI assistants

[Documentation](index.md) · [Русский](../ru/collaboration.md) · [Project status](status.md)

Two people can work on Forma from two PCs, each with their own ChatGPT/Codex session. GitHub carries source changes, task assignments and reviews. Each person needs a separate local clone, their own GitHub account and access to the repository. A chat session cannot see another chat's uncommitted files or private conversation; it must read the repository and the assigned Issue.

This is a development workflow, not simultaneous editing of a CAD project. Keep application databases, personal models and credentials out of Git. Use the application's checked transfer workflow described in [Projects](projects.md) when transferring a model. The workflow does not declare full 2.0 readiness or configure branch protection automatically.

## Recommended mode: take turns

For this team, the repository owner is **lovlyDev** and the friend's account is **wenlixxq**. These names identify the handoff participants; invitation and write access must be checked separately. This guide does not establish that access has already been granted. The other participant reviews the work, while turn and release ownership are assigned explicitly.

Both people may fetch and push changes, but editing ownership is handed over explicitly: person A first, then person B. While A works, B may read and review but does not begin another write task in the shared code. The active person's local subagents may work within the claimed task and separate file ownership; a second PC does not automatically gain editing rights. GitHub does not enforce this queue: the humans confirm the handoff in the Issue or their agreed channel.

1. A claims the Issue, branch and paths and records the base SHA. B confirms that they are currently reading/reviewing only.
2. A implements the task, reviews the diff, runs applicable checks, pushes small commits to the task branch and opens a PR. Push authority covers the agreed task; release publication is assigned separately.
3. B reviews the PR. If changes are needed, A retains the turn. After the agreed review, the designated integrator merges the PR. Unverified work is not handed over as a finished `main`.
4. A says: “Turn handed to B; Issue #…, merged PR …; main SHA …; checks …; limits …; next scope …; version/release owner …”. A stops editing until the next confirmed turn. Pushing a branch alone does not release the task.
5. B confirms the handoff, checks their clone is clean, gets current `main` and creates the next Issue's branch. After B's PR and review, the roles switch back.

On each PC, check the working tree **before** fetch/pull. With uncommitted changes, do not switch branches or pull over them; first preserve the work with its owner. For a clean tree and a confirmed turn:

```powershell
git status --short
git fetch origin
git switch main
git pull --ff-only origin main
git rev-parse HEAD
git switch -c task/456-friend-next-step
```

Compare the SHA with the handoff. Divergence makes `pull --ff-only` stop: agree on a resolution rather than replacing it with reset/force. If the PR has not merged, explicitly hand over its branch and SHA, not “main”. In that case confirm a new owner of the same task and continue that branch normally; do not run two authors concurrently or rewrite published history.

Assign one version/release integrator for every turn. Completed application changes require synchronized versions and a newly checked Windows EXE under AGENTS.md. Declare full 2.0 only after the complete acceptance gate in the [shared architecture plan](../architecture-2.0.md), and publish only through the assigned release owner. The [complete starter prompt for your friend's AI](collaboration-ai-prompt.md) includes these boundaries and source references. Chat memory and `.local` are not transferred by pull.

## Assign work before starting an agent

Create one GitHub Issue for each independently reviewable task. Assign one human owner, and record the branch, starting commit, permitted files/modules, expected behavior, exclusions and acceptance checks. A comment such as “Claimed by Anna; branch task/123-anna-section-ui; owns section controls and their tests; backend owned by Boris” makes responsibility visible. The second person reads open Issues and PRs before claiming another task.

An Issue assignment is a human coordination record, not an atomic distributed lock. If two people claim the same work, agree on one owner before editing. Split by clear module boundaries: one agent may own UI while another owns a Rust module, but agree on the API and ownership of shared files first. Reserve shared i18n files, lockfiles, migration numbers, version files and release metadata with a named owner. Share required text/API changes with that owner instead of editing simultaneously.

Use the repository's [Development task Issue form](../../.github/ISSUE_TEMPLATE/development-task.yml) when creating an Issue. It records the human owner, branch/base, owned paths, exclusions, API, acceptance checks, reviewer, release owner and handoff. The form does not assign an account automatically; set the human owner in GitHub's Assignees field and confirm the claim. For a plain-text Issue, use this body, replacing the example values:

```text
Owner: Anna
Task / branch: #123 / task/123-anna-section-ui
Base: <commit SHA>
Goal and acceptance: <observable behavior and checks>
Owned files/modules: <paths>
Dependencies and API: <related Issue, agreed contract>
Excluded work: <other owner's modules, versions, releases>
State: claimed / working / review / merged / blocked
Evidence and handoff: <commands, results, limits, next step>
Reviewer: Boris
Release owner: Anna
```

Update the Issue when blocked, handing off or finishing. Do not let both people independently implement the same acceptance criteria. A committed workboard is optional; Issues remain the task record in this workflow, so there is no second shared task-lock file to reconcile.

## Separate branches and worktrees

On a new PC, clone the authorized repository URL into a new directory. Do not clone over an existing working folder. The following PowerShell examples use `main`; substitute the team's actual integration branch. Run commands one at a time and stop on errors. They are instructions, not commands executed by this guide.

```powershell
git clone <repository-url> Forma
Set-Location Forma
git status --short
git fetch origin
git switch main
git pull --ff-only origin main
git switch -c task/123-anna-section-ui
```

For a second task on the same PC, create a separate worktree after claiming the new task. The destination must be new. Keep the original branch and its local files intact:

```powershell
git fetch origin
git worktree add -b task/124-boris-section-core ..\Forma-124 origin/main
Set-Location ..\Forma-124
```

Each worktree has its own source tree and branch; Git objects are shared. Install dependencies in that worktree using the [development guide](development.md). Coordinate expensive native builds on one PC so they do not compete for the same output/cache directories. Do not share a live working folder or SQLite database through file synchronization as a substitute for Git.

Before updating an existing task branch, inspect `git status --short`. If there are uncommitted files, finish a small intentional commit or preserve them with the human owner's agreement. Do not run destructive reset, clean, or blanket staging to make the tree appear clean. With a clean tree:

```powershell
git fetch origin
git rebase origin/main
```

Resolve conflicts in owned files and rerun relevant checks. Coordinate shared-file conflicts with their owner. Do not rewrite a branch another person is using. Prefer rebasing a private, unpushed task branch; if a published branch needs updated integration changes, agree with the reviewer on merging `origin/main` instead of force-pushing.

## Give each agent a bounded task

Copy this starter prompt into each person's session and replace the placeholders:

```text
Read AGENTS.md, docs/en/status.md, docs/en/collaboration.md and GitHub Issue #<number>.
This task is claimed by <human>; branch <branch>; base commit <SHA>.
Work only on <owned files/modules>; <other human/agent> owns <other modules>.
Implement <goal> and verify <acceptance checks>. Confirm the agreed API in the Issue.
Preserve the current branch and all existing local files. Do not overwrite other work.
If the Issue or API is unavailable, inspect local source read-only and report the missing
contract before editing dependent files. Do not invent another agent's progress.
Do not change shared versions, migrations, lockfiles or releases without the named owner's
assignment. Send required shared changes to <owner>. Add RU/EN documentation and texts
within the agreed ownership. Run the checks appropriate to this task.
Report changed files, actual commands/results, limitations and the next handoff step.
No GitHub publication, merge, tag, release or installation without this human's explicit
authorization. Release owner: <human>. Private .local notes are not the shared handoff.
```

The human checks that the agent is in the intended folder and branch. Issue access may require a connector or a pasted, sanitized Issue body; do not give an agent another person's credentials. Explicitly authorize sending an Issue comment or PR when that action is desired. Local subagents inherit the same ownership boundaries; their number does not make two independent chats coordinate automatically.

## Review and integrate

Use small commits with one purpose. Review the diff before staging explicit paths; never include local artifacts or credentials:

```powershell
git diff --check
git diff
git add docs/en/collaboration.md docs/ru/collaboration.md
git diff --cached
git commit -m "docs: explain two-PC collaboration"
```

After the human authorizes remote publication, push the task branch and open a PR against the integration branch:

```powershell
git push -u origin task/123-anna-section-ui
```

The repository's [PR template](../../.github/pull_request_template.md) provides those handoff fields. The PR should link its Issue and describe the concrete before/after behavior, changed scope, actual checks and remaining limits. Another human reviews the code and runs or examines meaningful checks before merging. Avoid relying solely on an AI's “done” message. The templates do not create Issues, send comments, change permissions or enforce reviews on their own.

Use the checks applicable to the change from [Development](development.md), including documentation, i18n and migration checks when relevant. [Quality](../../.github/workflows/ci.yml) runs automatically for PRs targeting `main` and pushes to `main`; manual dispatch and reusable workflow calls are also supported. The native-CAD matrix runs only on manual dispatch with its native option. Check the actual job results and arrange that additional native run when needed; do not treat a PR with missing or skipped required checks as passing.

## One release owner

The release owner integrates reviewed tasks, reconciles dependency changes and allocates new migration filenames without changing applied migrations. They synchronize npm workspace/lockfile, Cargo and Tauri versions, run the required release checks, build a new Windows installer, preserve older artifacts, and verify the resulting file. Task owners hand off completed work; the required version bump and installer build are coordinated once at integration, rather than competing edits from every agent.

Only that owner publishes the approved release. [Release](../../.github/workflows/release.yml) is triggered by pushing a new matching version tag and invokes quality/build jobs; a tag push is therefore a publication action, not an ordinary synchronization step. Never create duplicate tags or overwrite previous releases. Repository branch protection and required reviews can be configured by its administrator; this guide does not claim they are already enabled.

## Share evidence, keep private data local

`.local` handoffs, private logs and chat memory are machine-local. Put a sanitized summary in the Issue/PR: commit SHA, owned files, agreed API, tests actually run, unresolved problems and next owner. On the second PC, fetch the reviewed commits and reread that summary; do not copy a chat's assumed state.

Do not commit application databases, user CAD files, build caches, tokens, provider keys or signing keys. Use synthetic fixtures for tests and repository-approved artifact storage for release outputs. Before ending a session, leave the branch intact, record unfinished work and release the task explicitly if someone else should take over.
