# Spec: Strict Planning Prompts And Planning Templates

## 1. User Need

The framework's generated planning templates and Codex workflow prompts must be strict enough to drive high-quality automated development. The CLI must not pretend to produce a real plan; it only creates templates and handoffs for Codex to fill.

## 2. Background And Evidence

- Source: user feedback during framework development.
- Existing behavior: generated plan/spec/tasks were lightweight and some wording implied the CLI could create real plans.
- Desired behavior: `plan` creates a planning packet and all prompts tell Codex to fill implementation-grade detail.

## 3. Goals

| Goal ID | Goal | Depends On | Priority | Acceptance Signal |
| --- | --- | --- | --- | --- |
| G1 | Upgrade templates to strict Spec Kit-style planning artifacts. | None | Must | Templates require goals, dependencies, tests, risks, and quality gates. |
| G2 | Upgrade generated plans/specs/tasks to the same standard. | G1 | Must | `sandrone plan` emits detailed sections. |
| G3 | Make `plan` create a planning packet. | G1 | Must | `sandrone plan --name <change-name> --request_id <id>` creates templates under `docs/changes`. |
| G4 | Strengthen Codex workflow skill instructions. | G1, G2 | Must | Generated skill documents planning and implementation rules. |
| G5 | Add explicit Codex thread handoff prompts. | G4 | Must | Each change includes `codex-plan.md` and `thread-handoff.md`. |
| G6 | Preserve commit approval boundaries for new repositories. | G3, G5 | Must | `new` initializes an empty target repo without creating a target commit. |
| G7 | Enforce target project process requirements. | G4, G5 | Must | Generated plans and change docs require project docs, target project change docs, pre-commit, checks, and AI review when present. |
| G8 | Extract a real installable Codex skill. | G4 | Must | `skills/sandrone/SKILL.md` is tracked and copied into generated workspaces. |
| G9 | Clarify workspace naming rules. | G8 | Must | Clone mode allows arbitrary outer workspace names; new mode uses `--name` for the outer workspace convention and target repo name. |

## 4. Non-Goals

- Do not add a frontend.
- Do not add CI back.
- Do not add a full natural-language planning engine yet.

## 5. Acceptance Criteria

- [ ] `plan` creates templates and does not claim the real plan is complete.
- [ ] Generated plans mention goal dependencies, change surfaces, testing, breaking changes, rollback, and quality gates.
- [ ] Codex skill forbids unsafe shortcuts and hardcoded secrets.
- [ ] Codex skill requires visible approval at plan and change-doc gates.
- [ ] Generated user workspaces include thread handoff prompts for separate task threads.
- [ ] Generated user workspaces do not create this framework repository's `proposal.json`.
- [ ] From-scratch repositories can start implementation in an isolated worktree before any target commit exists.
- [ ] Generated artifacts require Codex to read target project documentation and satisfy project-internal requirements.
- [ ] Generated change docs record target project change docs, pre-commit, documentation checks, tests, and AI review results.
- [ ] The canonical workflow skill exists as a tracked `SKILL.md`, not only as a Rust string literal.
- [ ] Naming guidance distinguishes clone workspaces from from-scratch project names.
- [ ] Existing validation passes.

## 6. Open Questions

- Future: should plan completeness be machine-validated beyond required files?
