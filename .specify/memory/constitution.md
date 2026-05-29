# Project Constitution

This project follows a Spec Kit-style workflow with an additional proposal index for PR auditability.

## Core Principles

### I. Proposal-First Development

Every meaningful change MUST start with a proposal. A proposal captures the user-visible intent, implementation plan, task breakdown, and final change document.

### II. Durable Proposal Artifacts

Every proposal MUST live under:

```text
docs/proposals/YYYY-MM-DD/<proposal-id>/
```

Required files:

- `spec.md`
- `plan.md`
- `tasks.md`
- `plan.html`
- `change-doc.md`

### III. Machine-Readable Index

Every proposal MUST be listed in the root `proposal.json` file. The index is the source used by CI and future automation to discover proposal state.

### IV. Approval Before Implementation

Implementation MUST NOT begin until `spec.md`, `plan.md`, and `tasks.md` exist and the proposal has been approved by the user or maintainer.

### V. Isolated Worktrees

Each work item SHOULD use an independent git worktree and branch. Unrelated proposals MUST NOT share implementation branches.

### VI. Change Documentation

Every implementation PR MUST include `change-doc.md` describing the actual code changes, validation performed, risks, and follow-up work.

### VII. Automation Boundaries

Agents MAY generate specs, plans, tasks, worktrees, code changes, and change docs. Agents MUST NOT silently skip proposal artifacts, overwrite user changes, merge automatically, or create PRs without a change doc.

### VIII. Code Understanding Layer

For cloned or existing repositories, the framework SHOULD generate reusable CodeGraph documentation before planning substantial work. If a work item implies broad architectural, migration, or cross-cutting changes, agents SHOULD refresh CodeGraph before implementation.

### IX. Strict Planning Quality

Plans MUST be implementation-grade. A plan is not acceptable if it only says "inspect, implement, test" without concrete goals, dependencies, expected change surfaces, test strategy, and quality gates.

Every plan MUST include:

- goals and dependency order,
- detailed goal descriptions,
- expected modules/files after repository inspection,
- proposed code changes,
- breaking-change assessment,
- unit/integration/negative/security test strategy,
- rollback plan,
- approval checklist,
- explicit open questions when information is missing.

### X. Safe Code Standard

Production code MUST fail gracefully. In Rust, production `panic!`, `.unwrap()`, and `.expect()` are forbidden unless the invariant is explicitly documented as unreachable and the scope is narrow. Test code MAY use them when idiomatic.

All languages MUST avoid:

- hardcoded API keys, tokens, credentials, personal paths, and environment-specific values,
- logging or documenting secrets,
- broad lint suppressions,
- unrelated refactors,
- hidden external dependencies.

### XI. Target Project Contract

When this framework is used against a target repository, agents MUST read and follow that repository's own development requirements before delivery. This includes project documentation, change doc rules, pre-commit scripts or hooks, documentation checks, format/lint/test commands, release note requirements, and AI review processes when present.

The framework change doc MUST record:

- target project documentation that was read,
- target project change doc path and completion status, when required,
- pre-commit command and result, when required,
- documentation, formatting, lint, build, and test commands and results,
- AI review findings and resolution status, when required,
- an explicit statement that all discovered target project requirements are complete or blocked with reasons.

## Proposal Status Values

- `draft`
- `planned`
- `approved`
- `implemented`
- `merged`
- `blocked`

## Pull Request Gate

A PR is ready only when:

- `cargo fmt --check` passes.
- `cargo check` passes.
- language-specific static checks/lints pass when available.
- `python3 scripts/validate_proposals.py` passes.
- All proposal artifacts exist.
- `proposal.json` is updated.

## Governance

Changes to this constitution MUST themselves be represented by a proposal.
