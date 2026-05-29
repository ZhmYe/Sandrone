# Codex Auto Dev Workflow

Rust CLI framework for wrapping any git repository with a Codex-friendly development workflow.

## MVP Flow

```text
issue update -> plan -> approval -> isolated worktree -> Codex implementation -> change doc
```

The installed command is `codex-auto-dev`. It creates an outer workflow workspace around a target repository cloned into `dev/repo`.

## Build

```bash
cargo build
```

## Commands

```bash
cargo run -- init https://github.com/owner/repo.git
cargo run -- new my-new-project
cargo run -- update
cargo run -- request "Build first feature" "Detailed requirement"
cargo run -- list
cargo run -- codegraph --refresh
cargo run -- plan CAD-0001
cargo run -- approve CAD-0001
cargo run -- start CAD-0001
cargo run -- github-create my-new-project --private
cargo run -- push "Initial implementation"
cargo run -- tick
cargo run -- validate
```

After `cargo install`, the same commands become:

```bash
codex-auto-dev init https://github.com/owner/repo.git
codex-auto-dev new my-new-project
codex-auto-dev tick
```

## Notes

- `new` creates a from-scratch target repository under `dev/repo`.
- `request` creates a manual work item when there is no external issue platform yet.
- `github-create` and `push` call `gh` and `git`; failures are reported as command errors.
- `codegraph` generates reusable repository understanding under `docs/codegraph/`.
- `tools/issue-update.sh` is the replaceable issue connector. The default implementation uses GitHub through `gh`.
- `skills/codex-auto-dev-workflow/SKILL.md` is the Codex operating guide for this workspace.
- `plan` produces proposal artifacts under `docs/proposals/YYYY-MM-DD/<id>/`.
- `start` creates `dev/worktrees/<id>` on branch `codex/<id>`.
- Runtime state lives under `.codex-auto-dev/` and should stay out of git.

## Governance

This repository follows a Spec Kit-style workflow. The canonical constitution lives in `.specify/memory/constitution.md`.

Every PR must include:

- An updated `proposal.json`.
- A proposal folder under `docs/proposals/YYYY-MM-DD/<proposal-id>/`.
- `spec.md`, `plan.md`, `tasks.md`, `plan.html`, and `change-doc.md` in that proposal folder.

Validate locally:

```bash
cargo fmt --check
cargo check
python3 scripts/validate_proposals.py
```
