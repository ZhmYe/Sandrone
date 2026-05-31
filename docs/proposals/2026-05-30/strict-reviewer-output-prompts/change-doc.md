# 变更文档: Strict Reviewer Output Prompts

## 摘要

本次变更扩写三个默认 reviewer prompt，让它们明确审查流程、输出协议、finding 格式、判定规则，并提供 approved、rejected、gate unavailable 三类完整 JSON 示例。同时默认 schema 要求 `gate_unavailable` 并禁止额外字段。

## 实现前后对比

- 实现前: reviewer prompt 只简短列出字段，缺少 JSON 示例和严格输出边界，LLM 可能输出 Markdown 或遗漏字段。
- 实现后: 每个 reviewer 都有完整输出协议和示例，默认 schema 也强制 `gate_unavailable` 和无额外字段。

## 关键设计点

### 统一输出协议

三个 reviewer 都要求只输出一个 JSON 对象，不得输出 Markdown、代码块或解释性文本。字段清单和 decision/approved/gate_unavailable 的关系在 prompt 中逐项说明。

### Reviewer-Specific 检查

PlanReviewer 聚焦计划完整性、依赖顺序、目标项目要求和审批门禁。TestReviewer 聚焦测试覆盖、失败路径、验证证据和测试弱化风险。DesignReviewer 聚焦需求完成度、approved plan 一致性、安全、兼容性和绕过门禁风险。

### 示例驱动稳定性

每个 prompt 都内置 approved、rejected 和 gate unavailable 示例，帮助 LLM 区分质量拒绝和门禁不可用，减少无效重试和格式漂移。

## 变更范围摘要

改动集中在默认 reviewer prompt、review schema、README/skill 文档、默认 asset 测试和本次 proposal artifacts。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`

## 风险与后续

- prompt 变长会增加 reviewer 调用上下文，但换来更稳定的机器输出。
- 后续可以把公共输出协议抽成单独模板，减少三份 prompt 的重复维护。
