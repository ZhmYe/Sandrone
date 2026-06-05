# 计划: Outer Tick Review State Machine

## 目标与依赖顺序

1. 修 strict review schema。
   先调整 `tools/schemas/review-result.schema.json` 的默认模板，再同步 fallback JSON、测试 fixture 和 reviewer prompt。后续 tick 状态机依赖 reviewer 输出稳定。

2. 拆分 agent phase。
   新增 `AgentPhase`、`plan-agent.md`、`implementation-agent.md`，让默认 `issue-agent.sh` 根据 `SANDRONE_AGENT_PHASE` 选择提示词。

3. 重构 tick 状态机。
   `tick` 负责 update、刷新 agent exit、提交 gate、运行 reviewer、创建 worktree、派发下一 phase。子 agent 只写当前 phase 产物。

4. 收紧文档和测试。
   更新 README、skill、runtime 模板和集成测试，确保后续 Codex 不再按旧的嵌套 reviewer 流程执行。

## 代码改动位置

- `src/main.rs`
  - review schema 默认模板和 reviewer 输出规范。
  - `tick` 状态机、agent phase、outer review gate。
  - 默认 `tools/issue-agent.sh`、`plan-agent.md`、`implementation-agent.md`。
  - upgrade 逻辑补齐新 prompt。
- `tests/cli_flow.rs`
  - 新 workspace asset 断言。
  - tick planning/implementation phase 集成测试。
  - strict schema 和 fallback fixture 断言。
- `README.md`
  - 自动流程和 connector contract。
- `skills/sandrone/SKILL.md`
  - Codex 使用 skill 时必须遵守的新流程。

## 测试策略

- `cargo fmt --check`
- `cargo check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `python3 scripts/validate_proposals.py`
- `git diff --check`

## 风险与回滚

- 风险: 旧 workspace 已经存在自定义 `tools/issue-agent.sh`，不会被 upgrade 覆盖。缓解: upgrade 会补齐缺失 prompt，文档说明 connector 可替换。
- 风险: review attempt 最大次数按 review detail attempt 计算，可能需要后续更精细的 retry policy。缓解: 当前先防无限重试，blocked 后可用 recovery 恢复。
- 回滚: 恢复旧 tick 逻辑和旧 prompt，但会重新暴露嵌套 reviewer 的网络问题，不建议回滚。
