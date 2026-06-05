# Spec: Runtime Status Sync And Detailed Review Findings

## 背景

自动流程中存在两个需要收紧的问题。第一，`advance` 以 `.sandrone/state/requests.tsv` 为主要状态源，如果该索引落后于 `docs/changes/<name>/status.json`，可能把已经进入 implementation 或 change-doc review 的 request 当成 `plan-submitted` 重新处理，导致重复 plan-review 和重复 implementation 派发。第二，reviewer finding 只有 `required_fix` 时仍可能过于笼统，下一轮 agent 不一定知道具体怎么改和怎么验证。

## 目标

- `advance` 和 `tick` 在处理 request 前，从 runtime `status.json` 同步更靠后的状态、branch 和 worktree。
- 如果 `status.json` 已经是 `change-doc-submitted`，即使中央索引仍是 `plan-submitted`，也必须进入 code-review，不能重跑 plan-review/start。
- reviewer finding 必须包含影响、必要修复、具体修改建议和验证方式。
- 旧的含糊 finding JSON 必须被 review gate 判为 invalid，并转成 blocking `gate_unavailable`。

## 非目标

- 不引入数据库或替换 TSV 状态文件。
- 不改变 reviewer 的三方结构或 recommended_next_phase 语义。
- 不自动合并或删除历史重复 agent 日志。

## 行为要求

- `status.json` 的 request_id 必须匹配，才允许同步回中央索引。
- 只允许 runtime 状态推进中央状态，不允许把中央状态回退。
- 同步 branch/worktree 时只填补中央索引中的空值。
- 每个 finding 必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。
- `critical/high` finding 的 `suggested_change` 必须足够具体，能指导下一轮 agent 修改文件、测试、计划或文档。

## 验证

- 构造中央索引 stale 的 request，确认 `advance` 不重跑 PlanReviewer、不派发重复 implementation，而是直接 code-review。
- 构造缺少详细修改建议的 reviewer JSON，确认 gate unavailable 并写入 fallback detail。
- 全量运行 Rust 测试、proposal 校验和 diff 检查。
