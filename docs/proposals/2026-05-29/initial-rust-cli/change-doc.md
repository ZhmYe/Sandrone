# Change Doc: Initial Rust CLI Workflow

## Summary

This change initializes the project as a Rust CLI framework for wrapping any git repository with a Codex-friendly issue-to-worktree workflow.

## Changes

- Replaces the initial Node.js CLI prototype with a Rust binary named `codex-auto-dev`.
- Adds `init <git-url>` to clone target repositories into `dev/repo`.
- Adds `new <project-name>` for from-scratch projects.
- Adds `request <title> [body]` for manual user requirements.
- Adds `github-create` and `push` helpers for GitHub-backed project setup.
- Adds `codegraph [--refresh]` and automatic best-effort CodeGraph docs generation.
- Adds `update`, `tick`, `plan`, `approve`, `start`, and `validate` workflow commands.
- Adds default `tools/issue-update.sh` as a replaceable issue connector.
- Adds default `skills/codex-auto-dev-workflow/SKILL.md` as the Codex operating guide.
- Adds `docs/constitution.md` to define repository contribution and automation rules.
- Adds `proposal.json` as the root proposal index.
- Adds `scripts/validate_proposals.py` to enforce proposal artifact requirements.
- Removes GitHub Actions CI for now; validation remains available locally.

## Validation

- `cargo fmt --check`
- `cargo check`
- `python3 scripts/validate_proposals.py`

## Risks

- The current Rust CLI intentionally uses a simple TSV store instead of a database.
- The default issue tool and GitHub helpers shell out to `gh`, so they depend on GitHub CLI authentication.
- CodeGraph integration depends on the `codegraph` executable being available.
- `start` prepares an isolated worktree and Codex instructions; actual implementation is performed by Codex using the generated skill.
