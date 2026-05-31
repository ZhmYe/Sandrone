# 变更文档: Review Gate Unavailable Block

## 摘要

本次变更把 reviewer 后端故障从普通 review rejected 中拆出来，作为 `gate_unavailable` 处理。只要 reviewer 脚本缺失、失败、空输出、非法 JSON，或自定义 reviewer 明确返回 `gate_unavailable: true`，request 会立即进入 `blocked`，issue-agent 不能继续重试或绕过门禁。

## 实现前后对比

- 实现前: `PlanReviewer` 脚本失败会生成 `review tool failed` 的 blocking JSON，但 request 仍是 `plan-review-rejected`，issue-agent 可能继续修改计划或尝试绕过。
- 实现后: reviewer gate 不可用会写入诊断、summary、status 和 recovery，并把 request 标记为 `blocked`。只有 reviewer 正常运行并给出 critical/high 时，agent 才进入修复循环。

## 关键设计点

### Gate Unavailable 是独立状态

`ReviewResult` 增加 `gate_unavailable` 和 `diagnostic`。summary 里会显示 reviewer、是否 blocking、是否 gate unavailable、诊断摘录和 detail 路径，方便用户快速看到失败原因。

### CLI 负责硬阻断

`plan-review` 和 `code-review` 不再把 reviewer 后端故障交给 issue-agent 猜测。CLI 检测到 gate unavailable 后直接调用 `mark_blocked`，生成 `recovery.md`，并输出包含 reviewer 名称和诊断的错误。

### Issue Agent 不得绕过

默认 issue-agent 脚本契约和 prompt 明确禁止调用 `codex-auto-dev approve/reject`，禁止手写或修改 approval JSON，禁止修改 reviewer 脚本或 schema 来绕过门禁。发现 `gate_unavailable: true` 必须 block。

## 变更范围摘要

改动集中在 review 执行、summary 输出、issue-agent prompt、review schema、README/skill 文档和集成测试。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`

## 风险与后续

- 旧 workspace 需要 `codex-auto-dev upgrade` 才能获得更新后的默认 prompt 和 schema；用户自定义 reviewer 不会被覆盖。
- 如果 reviewer 后端经常不可用，应该修复 connector 或提供稳定的替代后端，而不是让 issue-agent 跳过 gate。
