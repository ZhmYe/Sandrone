# Plan: Strict Planning Prompts And Planning Templates

## 1. Summary

Upgrade the framework's planning layer from lightweight scaffolding to implementation-grade prompts and templates. The CLI creates planning packets; Codex fills the real plan after reading the request and target repository.

## 2. Goal Dependency Graph

| Order | Goal ID | Description | Depends On | Why This Order |
| --- | --- | --- | --- | --- |
| 1 | G1 | Strengthen reusable templates. | None | Templates define the standard all future artifacts follow. |
| 2 | G2 | Strengthen runtime generated spec/plan/tasks/change doc. | G1 | Runtime artifacts should match template expectations. |
| 3 | G3 | Change `plan` to create a planning packet. | G2 | Planning artifacts should exist before Codex fills them. |
| 4 | G4 | Strengthen generated Codex skill. | G1, G2 | Codex needs explicit operational constraints. |

## 3. Planned Code Changes

| Path / Module | Change Type | Description | Breaking? | Tests |
| --- | --- | --- | --- | --- |
| `src/main.rs` | modify | Make `plan` create templates under `docs/changes`, upgrade generated spec/plan/tasks/change-doc and skill text, avoid writing runtime `proposal.json`, avoid automatic target commits, and require target project process compliance. | Yes, command semantics change. | CLI integration tests for new/update/plan/start/finish. |
| `.specify/templates/*` | modify | Replace placeholder templates with strict implementation-grade templates and project-internal requirement sections. | No | Proposal validation. |
| `.specify/memory/constitution.md` | modify | Add strict planning, safe code, and target project contract standards. | No | Proposal validation. |
| `README.md` | modify | Document stricter `new` and planning standards. | No | Manual review. |
| `skills/codex-auto-dev-workflow/SKILL.md` | add | Store the canonical installable Codex skill as a tracked file. | No | Generated workspace skill equality test. |
| `scripts/install.sh` | add | Install the tracked skill into Codex home and optionally install the CLI. | No | Installer integration test with temporary Codex home. |
| `scripts/bootstrap.sh` | add | Clone/update this repo from GitHub and run `scripts/install.sh --force` for one-command remote setup. | No | Bootstrap help test. |
| `tests/cli_flow.rs` | add | Cover the from-scratch CLI flow, orphan worktree creation, and explicit GitHub repo name requirement. | No | `cargo test`. |

## 4. Testing Strategy

- `cargo fmt --check`
- `cargo check`
- `python3 scripts/validate_proposals.py`
- `cargo test`
- Smoke test from-scratch flow: `new --name` creates only framework and empty target repo.
- Smoke test approval/start flow: `plan`, `start`, verify an orphan worktree is created when the target repo has no commits.
- Integration assertion: generated plans, handoffs, skill, and change docs mention project-internal requirements, target project change docs, pre-commit, and AI review.
- Integration assertion: generated workspace skill exactly matches `skills/codex-auto-dev-workflow/SKILL.md`.
- Integration assertion: local installer copies the tracked skill into a Codex home directory.
- Integration assertion: bootstrap documents the remote one-command install path.

## 5. Quality Gates

- [ ] No production panic/unwrap/expect shortcuts introduced.
- [ ] No hardcoded secrets.
- [ ] No unrelated refactors.
- [ ] Generated text clearly states required tests and safety constraints.
- [ ] Generated text requires target project documentation review and records target project checks.

## 6. Breaking Changes And Migration

- Breaking change: `codex-auto-dev new` now treats trailing arguments as project brief, not base branch.
- Breaking change: public workflow commands are now focused on `new`, `update`, `plan`, `start`, and `finish`.
- Migration: existing users can edit `.codex-auto-dev/config.toml` if they need a non-main base branch for new projects.

## 7. Rollback

Revert this proposal's changes and restore the previous lightweight `new` behavior.
