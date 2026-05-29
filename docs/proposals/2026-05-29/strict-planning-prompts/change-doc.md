# Change Doc: Strict Planning Prompts And Planning Templates

## Summary

This change strengthens generated planning artifacts and Codex workflow instructions. The CLI now creates templates and handoffs while Codex remains responsible for filling real plans and change docs.

## Actual Changes

- Upgraded Spec Kit templates with strict sections for goals, dependencies, tests, risks, and quality gates.
- Updated the constitution with strict planning and safe-code standards.
- Updated `plan` to create planning templates under `docs/changes`.
- Stopped writing `proposal.json` into generated user workspaces; `proposal.json` remains only for this framework repository's own governance.
- Added `codex-plan.md` and `thread-handoff.md` generation for explicit Codex handoffs.
- Strengthened the generated skill so Codex must stop at plan approval and change-doc approval, and must not commit/push/PR without explicit user approval.
- Removed the automatic target repository scaffold commit from `new`.
- Added empty-repository `start` support through an orphan implementation worktree.
- Converged workflow wording around mechanical CLI steps and Codex-owned planning/implementation.
- Added CLI integration tests for from-scratch planning, orphan worktree start, and explicit GitHub repo name usage.
- Added target project process compliance requirements to generated plans, tasks, handoffs, skill text, and change docs.
- Added change-doc sections for target project change docs, pre-commit, documentation checks, tests, and AI review findings.
- Extracted the workflow guidance into `skills/codex-auto-dev-workflow/SKILL.md` so it can be installed as a real Codex skill.
- Updated the CLI to embed and copy the tracked skill file instead of maintaining a separate Rust string literal.
- Added `scripts/install.sh` for local one-command installation of the skill and CLI.
- Added `scripts/bootstrap.sh` for `curl -fsSL ... | sh` remote installation from GitHub.
- Updated `SKILL.md` to make CLI install/verification the required first step before workspace commands.
- Clarified workspace naming: clone mode may use any outer workspace name, while from-scratch mode should use `<name>-auto-dev` outside and the exact `--name` as the target repository name.
- Expanded generated spec, plan, tasks, change doc, and Codex skill text.
- Updated README guidance.

## Validation Evidence

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy -- -D warnings`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] Local installer test with temporary Codex home
- [x] Bootstrap help test
- [x] Skill install-first wording test
- [x] Workspace naming assertions for clone and from-scratch modes
- [x] Smoke test `codex-auto-dev new`
- [x] Smoke test `codex-auto-dev plan -> start`
- [x] Smoke test `codex-auto-dev finish`

## Risks And Follow-Ups

- Plan text is now longer and stricter; future work should add machine validation for plan completeness.
