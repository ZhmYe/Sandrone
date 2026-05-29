# Plan: [TITLE]

## 1. Summary

[One paragraph describing the planned implementation and why it satisfies the spec.]

## 2. Goal Dependency Graph

| Order | Goal ID | Description | Depends On | Why This Order |
| --- | --- | --- | --- | --- |
| 1 | G1 | [Goal] | None | [Reason] |

## 3. Design

- Architecture: [components/modules and responsibilities]
- Data model: [state, files, database tables, config]
- API/CLI surface: [commands, endpoints, function signatures]
- Error handling: [recoverable errors and user messages]
- Configuration: [env vars/config files; no hardcoded secrets]

## 4. Project-Internal Requirements

Read target project documentation before implementation and list every requirement that affects delivery.

| Requirement | Source File / Command | Required? | Evidence To Record |
| --- | --- | --- | --- |
| Target project documentation | [README/CONTRIBUTING/docs/etc.] | Yes | Documents read and rules summarized. |
| Target project change doc | [Path or "Not required"] | [Yes/No] | Path and completion status. |
| Pre-commit | [Command or hook] | [Yes/No] | Exact command and result. |
| Documentation checks | [Command] | [Yes/No] | Exact command and result. |
| Format/lint/test checks | [Commands] | [Yes/No] | Exact commands and results. |
| AI review | [Tool/command/process] | [Yes/No] | Findings and resolution status. |

## 5. Planned Code Changes

| Path / Module | Change Type | Description | Breaking? | Tests |
| --- | --- | --- | --- | --- |
| `[path]` | add/modify/delete | [Expected change] | No | [Test file/case] |

## 6. Implementation Steps

1. [Small reversible step]
2. [Next step]

## 7. Testing Strategy

Tests must cover success paths, validation failures, edge cases, and regression risks.

- Unit tests: [functions/modules and cases]
- Integration tests: [CLI/API/workflow cases]
- Negative tests: [invalid input, missing config, command failure]
- Security tests: [secret handling, unsafe input, auth failures]
- Project-required checks: [pre-commit, documentation checks, format/lint/test commands, AI review]
- Manual verification: [commands to run]

## 8. Quality Gates

- [ ] No production `panic!`, `.unwrap()`, or `.expect()` unless documented as unreachable and justified.
- [ ] No hardcoded API keys, tokens, credentials, personal paths, or environment-specific values.
- [ ] No unrelated refactors.
- [ ] Public behavior changes are documented.
- [ ] Breaking changes include migration notes.
- [ ] Language-specific formatter/linter/checker passes.
- [ ] Target project documentation has been read and followed.
- [ ] Target project change doc is completed when required.
- [ ] Required pre-commit, documentation checks, tests, and AI review are completed and recorded.

## 9. Breaking Changes And Migration

- Breaking change: [Yes/No]
- Migration required: [Steps or "None"]

## 10. Risks And Rollback

- Risk: [Risk]
- Mitigation: [Mitigation]
- Rollback: [How to revert safely]
