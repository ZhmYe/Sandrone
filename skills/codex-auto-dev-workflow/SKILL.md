---
name: codex-auto-dev-workflow
description: Use when the user asks Codex to create, clone, update, plan, implement, finish, or manage software work with codex-auto-dev workspaces, especially when approval gates, request IDs, change templates, isolated worktrees, target project checks, or no-commit/no-push boundaries matter.
metadata:
  short-description: Run codex-auto-dev approval-gated workspaces
---

# Codex Auto Dev Workflow

Use this skill whenever the user asks Codex to create, clone, update, plan, implement, or finish work in a `codex-auto-dev` workspace.

## Required First Step: Install Or Verify CLI

Before any workspace command, verify that the CLI is installed:

```bash
codex-auto-dev --help
```

If the command is missing, stop and tell the user that this skill also needs the Rust CLI. After explicit user approval, install both the CLI and local skill copy with:

```bash
curl -fsSL https://raw.githubusercontent.com/ZhmYe/codex-auto-dev-workflow/master/scripts/bootstrap.sh | sh
```

For a local clone, use:

```bash
scripts/install.sh --force
```

Do not run workspace commands until `codex-auto-dev --help` succeeds.

## Core Boundary

`codex-auto-dev` is mechanical. It creates the outer framework, target repository, request records, change folders, templates, handoff prompts, worktrees, and status markers.

Codex does the thinking. Codex must fill plans, implement code, write change docs, run checks, and report approval gates.

## Non-Negotiable Gates

- Never invent a repository name. Use exactly the name or URL the user provided.
- Never treat generated templates as a completed plan.
- Never start implementation until the user has approved the filled `plan.md`.
- Never finish until the user has approved the filled `change-doc.md`.
- Never run `git commit`, `git push`, create a PR, or merge unless the user explicitly requests it after change-doc approval.
- Never edit `dev/repo` directly during implementation. Work only in `dev/worktrees/<request_id>`.
- Each request should have its own Codex thread. Use `thread-handoff.md` unless the user explicitly asks to continue in the current thread.
- Never mark implementation complete until target project documentation and project-internal requirements have been read, satisfied, and recorded in `change-doc.md`.

## Commands

### Create Workspace

For an existing repository:

```bash
codex-auto-dev new --url <git-url>
```

For cloned repositories, the outer workspace directory name is not important. Use the user-specified directory when provided; otherwise choose a practical folder such as `<repo-name>-auto-dev`.

For a from-scratch repository:

```bash
codex-auto-dev new --name <exact-project-name>
```

For from-scratch repositories, the outer workspace and inner target git repository must both derive from the exact `--name` value. If the user says `--name my-app`, create or enter an outer workspace named `my-app-auto-dev`, then run `codex-auto-dev new --name my-app`. Do not invent a different project name.

Report the workspace path, `dev/repo`, `tools/issue-update.sh`, and this skill path. Do not create a plan unless the user asks for a specific request to be planned.

### Update Requests

```bash
codex-auto-dev update
```

This runs `tools/issue-update.sh`. The script is replaceable and must emit stable external IDs so repeated updates do not duplicate requests.

### Create Planning Packet

```bash
codex-auto-dev plan --name <YYYY-MM-DD-short-english-name> --request_id <REQ-0001>
```

This creates `docs/changes/<YYYY-MM-DD-short-english-name>/` with templates only. Codex must fill the templates by reading the request, target repository, and target project docs.

Planning output must include:

- concrete goals and dependency order,
- exact change surfaces,
- project-internal requirements and sources,
- target project change doc requirement,
- pre-commit, documentation check, format/lint/test, and AI review requirements,
- test strategy,
- risks, rollback, and open questions.

Stop after planning and give the user `plan.md` for approval.

### Start Implementation

```bash
codex-auto-dev start --request_id <REQ-0001>
```

This creates or reuses an isolated worktree and writes implementation handoff prompts. Codex then implements inside the worktree only.

Implementation must:

- follow the approved plan,
- satisfy target project requirements,
- complete target project change docs when required,
- run required pre-commit, documentation checks, formatters, tests, and AI review,
- fill `change-doc.md` with exact evidence,
- stop for user approval.

### Finish

```bash
codex-auto-dev finish --request_id <REQ-0001>
```

Run only after the user approves the change doc. This marks status; it does not commit, push, create a PR, or merge.

## Completion Response

When implementation is complete, report:

- change doc path,
- worktree path,
- branch,
- project-internal requirements completed,
- target project change doc path or `Not required`,
- pre-commit, documentation check, test, and AI review results,
- statement that no commit/push/PR was performed.
