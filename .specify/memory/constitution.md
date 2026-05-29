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
- `python3 scripts/validate_proposals.py` passes.
- All proposal artifacts exist.
- `proposal.json` is updated.

## Governance

Changes to this constitution MUST themselves be represented by a proposal.
