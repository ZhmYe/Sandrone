# Spec: Initial Rust CLI Workflow

## User Need

The project should start as a terminal-first Rust CLI framework that wraps any git repository with a Codex-friendly development workflow.

## Scope

- Create a Rust binary named `sandrone`.
- Support `init <git-url>` to clone a target repository into `dev/repo`.
- Support `new <project-name>` for from-scratch project creation.
- Support manual requirements through `request <title> [body]`.
- Support GitHub repository creation and pushing through `gh`/`git`.
- Generate CodeGraph documentation for cloned or created repositories.
- Generate replaceable tools and Codex workflow skill files.
- Support basic issue lifecycle commands.
- Generate plan and change document artifacts.
- Establish Spec Kit-style governance.
- Add CI checks for Rust and proposal artifacts.

## Non-Goals

- Build a frontend UI.
- Add a database service.
- Fully automate implementation without Codex participation.
- Automatically merge pull requests.

## Acceptance Criteria

- [x] `cargo check` passes.
- [x] `cargo fmt --check` passes.
- [x] A proposal index exists at `proposal.json`.
- [x] The framework creates `tools/issue-update.sh`.
- [x] The framework creates `skills/sandrone/SKILL.md`.
- [x] The framework supports from-scratch projects through `new`.
- [x] The framework supports manual requirements through `request`.
- [x] The framework supports CodeGraph documentation generation.
- [x] This proposal includes `spec.md`, `plan.md`, `tasks.md`, `plan.html`, and `change-doc.md`.
- [x] CI validates Rust checks and proposal artifacts.

## Open Questions

- How directly should the CLI invoke Spec Kit commands versus generating compatible artifacts itself?
- Which persistent database should replace the local TSV store after the CLI workflow is proven?
