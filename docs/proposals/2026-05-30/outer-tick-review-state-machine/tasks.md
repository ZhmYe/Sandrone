# 任务: Outer Tick Review State Machine

- [x] 修正 review result schema 的 required 字段和 finding required 字段。
- [x] 补齐 reviewer fallback JSON、测试 fixture 和 prompt 中的 `gate_unavailable` 与 finding 字段。
- [x] 新增 planning / implementation agent phase。
- [x] 新增 `plan-agent.md` 和 `implementation-agent.md` 默认提示词。
- [x] 修改 `tools/issue-agent.sh` 默认模板，使 agent 不再运行 reviewer gate。
- [x] 重构 `tick`，由外层执行 plan-review、start、code-review 和 waiting-finish 状态推进。
- [x] 更新 README 和 skill。
- [x] 更新集成测试。
- [x] 运行完整验证并把结果写入 change-doc。
