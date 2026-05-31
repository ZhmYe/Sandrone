# 规格: Strict Reviewer Output Prompts

## 背景

reviewer gate 是自动流程的质量核心。已有提示词已经规定了检查方向，但输出格式说明偏短，缺少完整 JSON 示例，容易让 LLM 输出 Markdown、遗漏字段、把 gate unavailable 和普通拒绝混在一起。

## 用户目标

让 PlanReviewer、TestReviewer 和 DesignReviewer 的提示词更精细，尤其是输出格式必须明确、稳定、可机器解析，并给出 approved、rejected 和 gate unavailable 的完整示例。

## 功能要求

- 三个 reviewer prompt 必须包含统一的输出协议。
- 输出协议必须说明只能输出一个 JSON 对象，不得输出 Markdown、代码块或前后缀文本。
- 输出字段必须明确包含 `reviewer`、`approved`、`gate_unavailable`、`decision`、`summary`、`process`、`critical`、`high`、`warning` 和 `info`。
- Finding 格式必须说明 `title`、`evidence`、`required_fix` 的含义。
- 每个 reviewer prompt 必须包含 approved、rejected、gate unavailable 三类完整 JSON 示例。
- schema 必须要求 `gate_unavailable`，并禁止额外字段。
- 文档和 skill 必须同步说明 reviewer 输出契约。

## 非目标

- 不改变 reviewer 正常 critical/high 的阻断语义。
- 不改变自定义 reviewer 脚本的兼容性；CLI 仍将缺失 `gate_unavailable` 的旧脚本视为 false。
- 不引入新的 JSON 解析依赖。

## 验收标准

- 新 workspace 生成的三个 prompt 都包含输出协议和示例。
- 默认 review schema 要求 `gate_unavailable` 并禁止额外字段。
- 现有 review gate、tick、finish 测试继续通过。
