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

`change-doc.md` SHOULD explain how the requirement was implemented rather than enumerate every changed file. It MUST include an implementation-before/after comparison, key design decisions with enough detail for review, a concise change-scope summary, validation evidence, risks, and follow-ups.

### VII. Automation Boundaries

Agents MAY generate request docs, plans, worktrees, code changes, change docs, review summaries, journals, and recovery docs. Agents MUST NOT silently skip required artifacts, overwrite user changes, merge automatically, or create PRs without a change doc.

For generated runtime workspaces, issue-agent runs MUST NOT commit, push, create PRs, or merge. After `change-doc.md` approval, the framework CLI MAY perform finish-time delivery by committing the approved worktree, pushing the request branch, and creating or preparing a PR through the replaceable `tools/pr-create.sh` connector. Merge remains a human or repository-platform decision.

### VIII. Code Understanding Layer

For cloned or existing repositories, the framework SHOULD generate reusable CodeGraph documentation before planning substantial work. If a work item implies broad architectural, migration, or cross-cutting changes, agents SHOULD refresh CodeGraph before implementation.

Before creating a runtime plan, agents SHOULD check whether the target repository needs `git pull` and whether CodeGraph must be generated or refreshed. Planning MUST NOT proceed when the local target repository is known to be behind upstream.

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

### XII. Human-Facing Document Language

面向人的框架文档和 runtime 模板 SHOULD 默认使用中文，包括 `request.md`、`plan.md`、`change-doc.md`、`agent-journal.md`、`recovery.md` 和必要的状态说明。

机器字段、命令、路径、JSON/TOML key、结构体名称、状态枚举、日志、测试输出、错误输出和外部原文 SHOULD 保持原样。若目标项目要求英文文档，agents MUST 遵守目标项目要求，并在框架文档中用中文说明该要求与完成状态。

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
- `cargo test` passes with no ignored failure masking.
- Tests for intentional failure paths assert the expected error message or status text, not only non-zero exit status. Review-gate failure tests MUST match the rejected reviewer, stale approval, missing approval, or invalid structured output message.
- `python3 scripts/validate_proposals.py` passes.
- All proposal artifacts exist.
- `proposal.json` is updated.

## Governance

Changes to this constitution MUST themselves be represented by a proposal.
