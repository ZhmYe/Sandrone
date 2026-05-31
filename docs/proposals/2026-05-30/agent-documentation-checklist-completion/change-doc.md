# 变更文档: Agent Documentation Checklist Completion

## 摘要

本次变更把“implementation agent 完成开发后必须更新文档，并确保交付文档 checklist 全部闭合”的要求写入默认 prompt、runtime change-doc 模板、README 和 workflow skill。未来自动流程生成的工作区会直接携带这条规则。

## 实现前后对比

- 实现前: implementation agent prompt 要求填写 `change-doc.md` 和验证证据，但没有明确要求检查交付文档中的未勾选 checklist。无法由当前流程完成的事项可能留在 checklist 中，让审批和恢复时难以判断状态。
- 实现后: implementation agent 必须更新相关文档，检查交付文档 checklist，把不能由当前流程完成的事项移到后续流程、人工事项或阻塞项，并在 `change-doc.md` 中记录检查结果。

## 关键设计点

### Prompt 约束

`tools/prompts/implementation-agent.md` 的默认内容新增“文档与 checklist 要求”。它明确区分三类事项: 已完成并打勾、当前流程无法完成并移到后续流程、无法安全推进则 block。这样 agent 不会为了让 checklist 看起来完整而虚假勾选。

### Change Doc 模板

`change-doc.md` 模板新增“文档与 Checklist”和“后续流程”。前者记录更新过哪些文档、检查过哪些 checklist；后者记录人工审批、外部发布、账号权限、跨团队确认或后续版本事项。

### Approved Plan 边界

prompt 明确不要为了凑勾篡改已批准 plan。approved plan 是审批产物，implementation 阶段应该在最终 change-doc 中解释执行结果，而不是修改历史审批证据。

## 变更范围摘要

- 生成器: 默认 issue-agent prompt、implementation-agent prompt 和 change-doc 模板。
- 测试: 新增生成内容和 skill 文本断言。
- 文档: README、workflow skill、本 proposal。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、默认 prompt、runtime 文档模板和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 文档与 Checklist

- 已更新的文档: README、workflow skill、本 proposal、runtime `change-doc.md` 模板说明。
- 所有交付文档中的 checklist 是否已全部打勾: yes；检查范围包括本 proposal 的 `tasks.md`、本 `change-doc.md`、README 和 workflow skill。
- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: yes；本次没有剩余人工事项。

## 后续流程

本次没有需要保留的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。

## 验证证据

- TDD red: `cargo test --test cli_flow` 失败，缺少 skill、implementation prompt 和 change-doc 模板断言对应内容。
- TDD green: 更新生成器和文档后，`cargo test --test cli_flow` 通过，29 个集成测试全部通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，29 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 19 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
