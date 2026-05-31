# 计划: Review Gate Unavailable Block

## 目标依赖图

1. 区分 reviewer 拒绝与 gate 不可用。
   先在 review 执行结果中记录 `gate_unavailable` 和诊断信息。
2. gate 不可用时 block。
   依赖上一步识别结果，让 `plan-review` 和 `code-review` 在后端不可用时调用 `mark_blocked`。
3. 收紧 issue-agent 边界。
   依赖 CLI block 行为，更新默认脚本契约、prompt、skill 和 README。
4. 补充测试。
   覆盖 reviewer 脚本失败时的明确错误文本、summary、status 和 request 状态。

## 代码改动

- 修改 `src/main.rs`:
  - `ReviewResult` 增加 `gate_unavailable` 和 `diagnostic`。
  - reviewer 脚本缺失、失败、空输出、非法 JSON 时保留诊断并标记 gate unavailable。
  - `plan-review` / `code-review` 遇到 gate unavailable 时 block。
  - review schema 增加可选 `gate_unavailable` 字段。
  - issue-agent prompt 禁止调用 `approve/reject` 或修改 approval JSON。
- 修改 `tests/cli_flow.rs`:
  - 新增 reviewer backend failure 测试。
  - 扩展默认 asset 断言，确认脚本契约包含 gate unavailable 和禁止绕过。
- 修改 README 和 skill:
  - 说明 gate unavailable 会直接 block。

## 测试策略

- 用 shell 脚本模拟 `plan-review.sh` 后端离线并退出非 0。
- 断言 stderr 包含 `PlanReviewer review gate unavailable` 和后端诊断。
- 断言 review detail、summary、status 和 request state 都记录 block。
- 跑完整 `cargo test`、clippy、proposal 校验和 diff check。

## 风险与回滚

- 该变更会让 reviewer 后端故障更早停止自动化，这是预期行为。
- 如果用户需要临时绕过，应该修复或替换 reviewer connector，而不是让 issue-agent 手动 approval。
