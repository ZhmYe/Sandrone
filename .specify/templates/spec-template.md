# Spec: [TITLE]

## 1. User Need

[Describe the user problem, product goal, or operational need. Include the source issue/request ID and links when available.]

## 2. Background And Evidence

- Source: [manual request / GitHub issue / internal tracker]
- Repository context: [CodeGraph docs, files inspected, existing behavior]
- Assumptions: [Only assumptions that are explicitly justified]

## 3. Goals

List concrete goals. Each goal must be observable and testable.

| Goal ID | Goal | Depends On | Priority | Acceptance Signal |
| --- | --- | --- | --- | --- |
| G1 | [Goal] | None | Must | [Observable result] |

## 4. Non-Goals

- [Out of scope]

## 5. Functional Requirements

- FR1: [Required behavior]

## 6. Non-Functional Requirements

- Reliability: [Error handling, retry, failure mode]
- Security: [Secrets, authentication, sensitive data]
- Performance: [Expected scale/latency if relevant]
- Maintainability: [Modularity, configuration, no hardcoding]
- Process compliance: [Target project documentation, change doc, pre-commit, documentation checks, tests, AI review]

## 7. Constraints And Forbidden Work

- Do not hardcode API keys, tokens, credentials, personal paths, or environment-specific values.
- Do not introduce panics/crashes in production code. Rust `panic!`, `.unwrap()`, and `.expect()` are allowed only in tests or explicitly documented unreachable invariants.
- Do not modify unrelated behavior.
- Do not edit outside the approved worktree.
- Do not create or merge PRs unless explicitly requested.
- Do not deliver without satisfying documented target project requirements.

## 8. Acceptance Criteria

- [ ] [Observable outcome tied to a goal]
- [ ] Target project documentation has been read and followed.
- [ ] Target project change doc, pre-commit, documentation checks, tests, and AI review are completed when required.

## 9. Open Questions

- [Question, or "None"]
