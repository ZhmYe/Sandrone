# 变更文档: Outer Tick Review State Machine

## 摘要

本次变更修复 review schema 严格结构，并把自动流程从“子 issue-agent 内部嵌套运行 reviewer”调整为“外层 tick 主控 reviewer gate”。agent 现在分为 planning 和 implementation 两个 phase，只负责写计划或实现代码与 change-doc；review、start、状态推进都由外层 tick 执行。

## 实现前后对比

- 实现前: issue-agent 在一个子 Codex 中连续写 plan、运行 plan-review、start、实现、运行 code-review。内层 reviewer 容易因为沙盒和网络不可用失败，也很难在 UI 上区分 plan 与 implementation 的状态。
- 实现后: tick 先派发 planning agent；agent 退出后 tick 提交 plan gate 并运行 PlanReviewer；通过后 tick 创建 worktree 并派发 implementation agent；implementation 退出后 tick 提交 change-doc gate 并运行 TestReviewer 和 DesignReviewer；通过后进入 `waiting-finish`。

## 关键设计点

### Strict Review Schema

默认 review schema 移除非必要 `$schema`，顶层 required 包含 `reviewer`、`approved`、`gate_unavailable`、`decision`、`summary`、`process`、`critical`、`high`、`warning` 和 `info`。finding required 固定为 `title`、`evidence`、`required_fix`。

reviewer fallback JSON 和测试 fixture 同步补齐 `gate_unavailable`。非法 JSON、缺少必备字段或旧式 finding 输出会转成 blocking review，不会被误当作正常 rejected。

### Outer Tick Gate

`tick` 现在刷新 agent exit code 后执行对应 gate:

- planning agent 成功退出: tick 写 plan submitted approval，再运行 `plan-review`。
- plan-review 通过: tick 运行 `start` 创建 worktree，并继续派发 implementation agent。
- implementation agent 成功退出: tick 写 change-doc submitted approval，再运行 `code-review`。
- code-review 通过: tick 标记 `waiting-finish`。

reviewer rejected 会让状态停在 `plan-review-rejected` 或 `code-review-rejected`，下一次 tick 重新派发对应 phase agent 修复。超过最大 review attempt 后进入 blocked。

### Agent Prompt 拆分

默认 connector 仍是 `tools/issue-agent.sh`，但通过 `SANDRONE_AGENT_PHASE` 选择:

- `tools/prompts/plan-agent.md`: 只写 `plan.md`，不得运行 submit/review/start。
- `tools/prompts/implementation-agent.md`: 只在 worktree 中实现并写 `change-doc.md`，不得运行 submit/review/finish。
- `tools/prompts/issue-agent.md`: 通用边界契约，强调不得 commit、push、PR、修改 approval 或绕过 reviewer。

### 文档和升级

README 和 skill 已同步新流程。`upgrade` 会补齐缺失的 plan-agent / implementation-agent prompt，但仍不覆盖用户已有 connector 和 prompt。

## 变更范围摘要

- CLI tick 状态机和 agent phase。
- 默认 agent/reviewer/schema 模板。
- README、skill 和 runtime plan/change-doc 模板。
- 集成测试和 proposal 索引。

## 目标项目内部要求

- 已阅读的目标项目文档: README、skill、现有 proposal 和测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 均通过。
- AI review: Not required，本次由本会话自检并运行自动测试。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，22 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 12 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；验证依赖本地格式、编译、clippy、测试和 proposal 校验。
