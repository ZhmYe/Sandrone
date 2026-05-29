# Tasks: Strict Planning Prompts And Planning Templates

## Phase 1: Templates

- [x] Strengthen spec template.
- [x] Strengthen plan template.
- [x] Strengthen tasks template.
- [x] Strengthen constitution.

## Phase 2: Runtime Generation

- [x] Make `plan` create planning templates.
- [x] Upgrade generated `spec.md`.
- [x] Upgrade generated `plan.md`.
- [x] Upgrade generated `tasks.md`.
- [x] Upgrade generated `change-doc.md`.
- [x] Upgrade generated Codex skill.
- [x] Generate `codex-plan.md`.
- [x] Generate `thread-handoff.md`.
- [x] Require visible plan and change-doc approval gates in generated skill.
- [x] Avoid automatic target commits during `new`.
- [x] Support starting from an empty target repository with an orphan worktree.
- [x] Converge public workflow commands around template generation and worktree creation.
- [x] Add CLI integration tests for the strict workflow gates.
- [x] Require target project documentation and project-internal requirements in generated artifacts.
- [x] Require change docs to record target project change docs, pre-commit, documentation checks, tests, and AI review results.
- [x] Extract the workflow guidance into a real tracked `SKILL.md`.
- [x] Make the CLI copy the tracked skill into generated workspaces.
- [x] Add `scripts/install.sh` for local skill and CLI installation.
- [x] Add `scripts/bootstrap.sh` for remote one-command GitHub installation.
- [x] Make the skill explicitly require CLI installation or verification before workspace commands.
- [x] Document clone/new workspace naming rules in the skill and CLI output.

## Phase 3: Verification

- [x] Run formatter.
- [x] Run Rust checks.
- [x] Run Rust tests.
- [x] Run proposal validation.
- [x] Test local skill installer with a temporary Codex home.
- [x] Test bootstrap help output.
- [x] Smoke test `new`.
- [x] Smoke test `plan -> start`.
- [x] Smoke test `finish`.
