# Tasks: [TITLE]

## Phase 1: Planning

- [ ] Read `issue.md` and `spec.md`.
- [ ] Read target project documentation and capture project-internal requirements.
- [ ] Refresh CodeGraph if the change is architectural, cross-cutting, or migration-like.
- [ ] Finalize goal dependency order in `plan.md`.
- [ ] Identify exact files/modules expected to change.
- [ ] Define unit, integration, negative, and security test coverage.
- [ ] Identify required target project change docs, pre-commit, documentation checks, tests, and AI review.
- [ ] Get approval before code changes.

## Phase 2: Implementation

- [ ] Create or enter the isolated worktree.
- [ ] Re-read target project documentation inside the worktree.
- [ ] Implement goals in dependency order.
- [ ] Keep changes scoped to the approved plan.
- [ ] Complete target project change docs or release notes when required.
- [ ] Avoid production `panic!`, `.unwrap()`, and `.expect()` unless explicitly justified.
- [ ] Avoid hardcoded secrets, tokens, personal paths, and environment-specific values.

## Phase 3: Verification

- [ ] Run formatter.
- [ ] Run language-specific static checks/lints.
- [ ] Run unit tests.
- [ ] Run integration tests.
- [ ] Run negative/error-path tests.
- [ ] Run target project pre-commit when required.
- [ ] Run target project documentation checks when required.
- [ ] Run target project AI review when required.
- [ ] Inspect diff for unrelated changes and sensitive data.

## Phase 4: Documentation

- [ ] Update `change-doc.md` with actual files changed, validation output, risks, and follow-ups.
- [ ] Record all target project requirements, completion status, target project change doc path, pre-commit output, and AI review findings.
- [ ] Update `proposal.json`.
- [ ] Mark blocked items clearly if any requirement cannot be completed.
