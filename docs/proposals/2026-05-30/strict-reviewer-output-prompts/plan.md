# 计划: Strict Reviewer Output Prompts

## 目标依赖图

1. 收紧 schema。
   先要求默认 reviewer 输出 `gate_unavailable`，并用 `additionalProperties: false` 限制额外字段。
2. 扩写 reviewer prompt。
   为 PlanReviewer、TestReviewer、DesignReviewer 增加审查流程、输出协议、finding 格式、判定规则和三类 JSON 示例。
3. 同步文档和 skill。
   让使用者知道替换 reviewer backend 时也必须保留同等输出契约。
4. 补测试。
   验证默认 workspace 生成的 schema 和 prompt 包含新协议。

## 代码改动

- 修改 `src/main.rs`:
  - 扩写 `default_plan_review_prompt`。
  - 扩写 `default_test_review_prompt`。
  - 扩写 `default_design_review_prompt`。
  - 修改 `write_default_review_schema`。
  - 合成 reviewer failure JSON 时包含 `gate_unavailable: true`。
- 修改 `tests/cli_flow.rs`:
  - 断言默认 schema 包含 `gate_unavailable` 和 `additionalProperties: false`。
  - 断言默认 prompt 包含输出协议和 reviewer-specific 示例内容。
- 修改 README 和 skill:
  - 说明 reviewer JSON 字段和 finding 证据要求。

## 测试策略

- 先跑 `cargo fmt --check` 和 targeted asset/review tests。
- 再跑完整 `cargo check`、clippy、`cargo test`、proposal 校验和 diff check。

## 风险与回滚

- 默认 LLM reviewer 输出会更稳定，但 prompt 更长，单次 review token 消耗会增加。
- 旧自定义 reviewer 缺少 `gate_unavailable` 字段时 CLI 仍兼容；如果用户使用 schema 驱动的自定义 backend，需要同步 schema。
