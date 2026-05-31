# 规格: Outer Tick Review State Machine

## 背景

自动流程原本让 `issue-agent` 在子 Codex 中连续执行 planning、plan-review、start、implementation 和 code-review。这个结构会出现嵌套 Codex reviewer 的网络和沙盒问题，也让前端难以清晰展示 plan 与 implementation 的独立状态。

同时 `tools/schemas/review-result.schema.json` 需要符合 Codex structured output 的严格规则: 顶层对象必须显式列出 required 字段并禁止额外字段；finding 也必须强制包含 `title`、`evidence` 和 `required_fix`。

## 目标

- 修正 review result schema，使默认 reviewer 输出与 Codex structured output 严格规则一致。
- 所有 fallback JSON、测试 fixture 和 reviewer prompt 都必须包含 `gate_unavailable`。
- 所有 finding 必须包含 `title`、`evidence` 和 `required_fix`。
- 将 reviewer gate 从子 agent 中移到外层 `tick` 状态机。
- 将 agent phase 拆成 planning 和 implementation，便于恢复、并行和前端展示。
- 保持 agent 边界: agent 不得运行 reviewer、start、finish、commit、push、PR 或手写 approval。

## 非目标

- 不新增前端界面。
- 不实现长期 daemon。
- 不把 reviewer 固定为 Codex；reviewer connector 仍保持可替换。
- 不修改目标项目的提交、push 或 PR 策略。

## 验收标准

- 新 workspace 默认生成 `tools/prompts/plan-agent.md` 和 `tools/prompts/implementation-agent.md`。
- `tick` 可以派发 planning agent，刷新退出后由外层运行 `plan-review`，通过后创建 worktree 并派发 implementation agent。
- implementation agent 退出后，由外层运行 `code-review`，通过后标记 `waiting-finish`。
- reviewer backend 失败、非法 JSON、缺失 `gate_unavailable` 或旧 finding 结构都不能被误当作通过。
- 集成测试覆盖 tick phase 拆分、reviewer gate、schema 输出和 upgrade 补齐。
