# Agent Review Preflight Self Check Change Doc

## 摘要

本次变更强化默认 agent prompt，让 planning 和 implementation 在退出前必须先按 reviewer 标准自检。`issue-agent` 被明确为共享契约和 connector 组合，不是过时文件，因此保留并补充职责说明。

## 实现前后对比

变更前:

- `issue-agent.md` 说明了通用边界，但没有明确解释它是共享 agent 契约。
- planning 的自检清单较粗，没有要求逐项核对 PlanReviewer。
- implementation 没有单独的 code-review preflight 章节。
- README 没有解释 `issue-agent.sh`、`issue-agent.md` 和 phase prompt 的关系。

变更后:

- `issue-agent.md` 标题和正文明确为共享 agent 契约，并说明会与 phase-specific prompt 组合。
- planning 退出前必须做 `PlanReviewer 提交前自检`。
- implementation 退出前必须做 `Code Review 提交前自检`，分别覆盖 TestReviewer 和 DesignReviewer。
- 自检发现可能产生 critical/high 时，agent 必须先修复或 block，不能直接交给 reviewer。
- README 和 skill 同步说明职责边界和自检要求。

## 关键设计点

- 自检规则写进默认生成 prompt，而不是只写在 README，确保新 workspace 自动获得约束。
- 共享契约负责跨 phase 的硬边界和提交前自检原则，phase prompt 负责细化各阶段的具体检查项。
- 自检结果要求写入 `agent-journal.md`，implementation 阶段还要写入 `change-doc.md`，保证后续恢复和人类审查可追溯。
- 测试只断言关键语义文本，避免把整段 prompt 锁死，保留后续继续优化 prompt 的空间。

## 变更范围摘要

- `src/main.rs`: 更新默认 issue-agent、plan-agent 和 implementation-agent prompt。
- `tests/cli_flow.rs`: 增加默认 prompt 生成断言。
- `README.md`、`skills/sandrone/SKILL.md`: 增加 `issue-agent` 职责说明和 self-check 规则。
- `docs/proposals/2026-05-31/agent-review-preflight-self-check/`: 新增本次 proposal 文档。
- `proposal.json`: 登记本次 proposal。

## 验证证据

- `cargo test new_name_creates_framework_and_empty_target_repo_only`: 已先观察到新增断言失败，再更新 prompt 后通过。
- `cargo fmt --check`: 通过。
- `cargo check`: 通过。
- `cargo test`: 通过，1 个单元测试和 40 个 CLI 集成测试通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，校验 30 个 proposal。
- `git diff --check`: 通过。

## Review 结果

本次变更没有运行自动 reviewer gate；以本仓库测试、clippy、proposal 校验和人工检查作为交付前验证。
