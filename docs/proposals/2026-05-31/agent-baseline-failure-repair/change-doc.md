# 变更文档: Agent Baseline Failure Repair

## 摘要

本次变更要求 implementation agent 修复测试过程中发现的已有失败，即使失败不是本分支直接引入。TestReviewer 也会检查这类 Baseline failure 是否被修复、复验并写入 change-doc。

## 实现前后对比

- 实现前: agent 可能把已有测试失败归类为非本分支问题后忽略。
- 实现后: agent 必须修复可安全处理的 Baseline failure，并记录失败命令、根因、修复范围和复验结果。
- 实现前: TestReviewer 主要审查新增实现测试覆盖。
- 实现后: TestReviewer 还会审查 Baseline failure 是否被修复；忽略这类失败会成为 high 或 critical finding。

## 关键设计点

### 当前 Worktree 内修复

Baseline failure 的修复仍发生在 request worktree 内，保留独立分支和 review gate。agent 不直接修改 `dev/repo`，也不跳过审批。

### 安全 Block 边界

如果修复会破坏 approved plan、需要外部权限/数据或无法安全判断，agent 必须 block 并写清恢复步骤。否则不能用“不是本分支改的”作为忽略理由。

### 文档化证据

change-doc 的验证证据必须包含 Baseline failure 小节，记录失败命令、根因、修复内容和复验结果，方便 reviewer 和用户判断修复是否可信。

## 变更范围摘要

- Prompt: implementation agent 和 TestReviewer 默认提示词。
- 文档: README、workflow skill、本 proposal。
- 测试: 默认 workspace 生成内容断言。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、agent/reviewer prompt 代码和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 文档与 Checklist

- 已更新的文档: README、workflow skill、本 proposal。
- 所有交付文档中的 checklist 是否已全部打勾: yes；检查范围包括本 proposal 的 `tasks.md`、本 `change-doc.md`、README 和 workflow skill。
- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: yes；本次没有剩余人工事项。

## 后续流程

本次没有需要保留的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。

## 验证证据

- TDD red: `cargo test --test cli_flow new_name_creates_framework_and_empty_target_repo_only -- --nocapture` 失败，默认 prompt 缺少 Baseline failure 规则。
- TDD green: 更新 implementation agent 和 TestReviewer prompt 后，上述测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，36 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 26 个 proposal。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
