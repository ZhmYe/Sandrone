# Codex Auto Dev Workflow

Rust CLI framework for wrapping any git repository with a Codex-friendly development workflow.

## MVP Flow

```text
workspace -> request update -> planning templates -> Codex fills plan -> approval -> worktree -> Codex implementation -> change doc -> approval -> finish
```

The installed command is `codex-auto-dev`. It creates an outer workflow workspace around a target repository under `dev/repo`. The CLI is deliberately mechanical: it creates folders, templates, state, and worktrees. Codex fills plans, writes code, runs project checks, and writes change docs.

## Build

```bash
cargo build
```

## Install

One-command install from GitHub:

```bash
curl -fsSL https://raw.githubusercontent.com/ZhmYe/codex-auto-dev-workflow/master/scripts/bootstrap.sh | sh
```

From a local clone, install both the Codex skill and CLI:

```bash
scripts/install.sh --force
```

Install only the Codex skill:

```bash
scripts/install.sh --skill-only --force
```

Install the skill from GitHub with Codex's skill installer:

```text
ZhmYe/codex-auto-dev-workflow skills/codex-auto-dev-workflow
```

Restart Codex after installing a skill.

## Commands

```bash
cargo run -- new --url https://github.com/owner/repo.git
cargo run -- new --name my-new-project
cargo run -- update
cargo run -- plan --name 2026-05-29-first-feature --request_id REQ-0001
cargo run -- start --request_id REQ-0001
cargo run -- finish --request_id REQ-0001
```

After `cargo install`, the same commands become:

```bash
codex-auto-dev new --url https://github.com/owner/repo.git
codex-auto-dev new --name my-new-project
codex-auto-dev update
codex-auto-dev plan --name 2026-05-29-first-feature --request_id REQ-0001
codex-auto-dev start --request_id REQ-0001
codex-auto-dev finish --request_id REQ-0001
```

## Notes

- `new --url <git-url>` clones an existing target repository into `dev/repo`.
- `new --name <project-name>` creates an empty target repository under `dev/repo` without committing target code.
- For clone mode, the outer workspace directory name is arbitrary. For `new --name`, use `<project-name>-auto-dev` as the outer workspace and the exact `--name` value as the inner target repository name.
- `tools/issue-update.sh` is the replaceable issue connector. The default implementation uses GitHub through `gh`.
- `update` runs the connector and records stable request IDs such as `REQ-0001`; repeated external IDs do not create duplicates.
- `skills/codex-auto-dev-workflow/SKILL.md` is the canonical Codex skill. The CLI copies this same tracked skill into generated workspaces.
- `plan --name <YYYY-MM-DD-short-name> --request_id <REQ-0001>` creates templates under `docs/changes/<YYYY-MM-DD-short-name>/`; it does not generate the real plan.
- `start --request_id <REQ-0001>` creates `dev/worktrees/<request_id>` on branch `codex/<request_id>`.
- For an empty from-scratch repository, `start` creates the first implementation branch as an orphan worktree.
- `finish --request_id <REQ-0001>` marks status after change-doc approval; it does not commit, push, create a PR, or merge.
- Runtime state lives under `.codex-auto-dev/` and should stay out of git.
- Generated changes include `codex-plan.md`, `codex-start.md`, and `thread-handoff.md` so Codex can run planning and implementation in separate, approval-gated threads.
- Codex must not commit, push, create PRs, or merge unless the user explicitly asks after reviewing `change-doc.md`.
- Codex must read and satisfy the target repository's own project-internal requirements, including target project change docs, pre-commit scripts, documentation checks, tests, and AI review when required.

## Codex Workflow

Use the skill:

```text
skills/codex-auto-dev-workflow/SKILL.md
```

Typical new-repository flow:

```text
User asks Codex: use the skill and create repo <exact-name>
Codex runs: codex-auto-dev new --name <exact-name>
User gives a request or asks to plan the initial project
Codex runs: codex-auto-dev plan --name <YYYY-MM-DD-short-name> --request_id REQ-0001
Codex fills: docs/changes/<YYYY-MM-DD-short-name>/plan.md
Codex stops: gives plan path and waits for approval
User approves
Codex runs: codex-auto-dev start --request_id REQ-0001
Codex starts or asks for a dedicated implementation thread
Codex updates: change-doc.md, including target project requirements, pre-commit, and AI review results
Codex stops: gives change doc path and waits for approval
User approves
Codex runs: codex-auto-dev finish --request_id REQ-0001
```

Typical clone flow:

```text
User asks Codex: use the skill and clone <url>
Codex runs: codex-auto-dev new --url <url>
User asks for update or a specific task
Codex runs: codex-auto-dev update
Codex creates a planning packet with codex-auto-dev plan
Codex fills the plan, waits for approval, then starts an isolated worktree only after approval
```

## Framework Repository Governance

This source repository follows a Spec Kit-style workflow. The canonical constitution lives in `.specify/memory/constitution.md`.

Generated user workspaces do not need this repository's `proposal.json`. They use `docs/changes/<YYYY-MM-DD-short-name>/` for runtime change artifacts.

Every PR against this framework repository must include:

- An updated `proposal.json`.
- A proposal folder under `docs/proposals/YYYY-MM-DD/<proposal-id>/`.
- `spec.md`, `plan.md`, `tasks.md`, `plan.html`, and `change-doc.md` in that proposal folder.

Validate locally:

```bash
cargo fmt --check
cargo check
python3 scripts/validate_proposals.py
```

Planning standards are intentionally strict, but the CLI only creates templates. Codex must fill generated plans with goal dependencies, expected change surfaces, project-internal requirements, testing strategy, breaking-change assessment, rollback, and quality gates such as no hardcoded secrets, no production panic/unwrap/expect shortcuts, required pre-commit checks, and AI review when required by the target project.
