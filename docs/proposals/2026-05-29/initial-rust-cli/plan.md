# Plan: Initial Rust CLI Workflow

## Goal

Initialize the project as a Rust CLI framework that can manage the first complete terminal-driven workflow around any cloned git repository:

```text
work item -> plan -> approval -> worktree -> change doc
```

## Scope

- Replace the initial JavaScript CLI prototype with a Rust binary named `codex-auto-dev`.
- Clone a target repository into `dev/repo`.
- Create a from-scratch target repository with `new <project-name>`.
- Accept manual user requirements with `request`.
- Provide GitHub create/push helpers that report command failures cleanly.
- Use CodeGraph as the reusable code understanding layer.
- Store local runtime state in `.codex-auto-dev/state/items.tsv`.
- Generate proposal artifacts under `docs/proposals/YYYY-MM-DD/<id>/`.
- Generate a replaceable issue update tool and Codex workflow skill.
- Establish durable governance artifacts under `docs/`.
- Add CI validation for Rust checks and proposal index integrity.

## Non-Goals

- No web frontend yet.
- No database service yet.
- No automatic PR creation yet.
- No direct Codex execution loop yet.

## Implementation Steps

1. Create `Cargo.toml` and `src/main.rs`.
2. Implement core CLI commands for init, issue update, planning, approval, worktrees, and change docs.
3. Implement new-project and manual-request paths.
4. Add CodeGraph docs generation.
5. Add a constitution for repository rules.
6. Add root `proposal.json`.
7. Add validation script.
8. Update README usage.

## Approval Notes

This proposal bootstraps the process that future changes must follow.
