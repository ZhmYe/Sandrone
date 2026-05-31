# Plan: Runtime Status Sync And Detailed Review Findings

## 目标与顺序

1. 先写状态不同步复现测试，把 `requests.tsv` 回退到 `plan-submitted`，保留 `status.json=change-doc-submitted`。
2. 实现 runtime status 同步 helper，在 refresh 和 dispatch 入口调用。
3. 写 reviewer finding 详细字段测试，确认旧格式被拒绝。
4. 扩展 review schema、fallback JSON、默认 reviewer prompt 和文档。
5. 更新 proposal 索引并运行完整验证。

## 实现位置

- `src/main.rs`: `sync_request_from_status_json`、状态 rank、review schema、fallback JSON、reviewer prompt。
- `tests/cli_flow.rs`: stale index 回归测试、finding 详细字段测试。
- `README.md`、`skills/codex-auto-dev-workflow/SKILL.md`: reviewer finding 契约说明。

## 状态同步设计

`status.json` 是 request 文档包中的 runtime 事实。同步时先验证 `request_id`，再比较状态 rank。只有 runtime 状态更靠后时才同步中央 `requests.tsv`，并只在中央 branch/worktree 为空时填补对应值。同步后追加 `request_state_synced` 事件。

## Reviewer 输出设计

`required_fix` 表示通过门禁的必要条件，`suggested_change` 表示具体修改建议，`verification` 表示修完如何证明。这样 agent 处理 reviewer 反馈时可以逐条写入 journal 和 change-doc，不需要猜 reviewer 想要什么。

## 测试策略

- stale index 测试必须确认 PlanReviewer 只运行一次。
- stale index 测试必须确认不会派发 duplicate implementation agent。
- finding 详细字段测试必须确认旧格式变成 blocking fallback JSON。
- 全量测试保证既有 tick/advance/review/finish 行为不回归。
