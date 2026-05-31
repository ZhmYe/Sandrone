# 规格: Strict Reviewer Gates

## 背景

无人值守自动化不能依赖可见 Codex session。计划生成、实现和审批需要转为可记录、可替换、可机器判断的 reviewer gate。用户要求 plan-review 和 code-review 都必须严格，且 reviewer 可配置，不绑定 Codex，未来可替换为 Claude Code、OpenAI API 或公司内部 LLM。

## 用户目标

框架需要提供 `plan-review` 和 `code-review` 命令。`plan-review` 调用 `PlanReviewer`，通过后自动 approval plan。`code-review` 调用 `TestReviewer` 和 `DesignReviewer`，两个都通过后自动 approval change-doc。任意 reviewer 出现 `critical` 或 `high`，或输出结构不合法，都必须阻断流程。

## 功能要求

- 新增 `codex-auto-dev plan-review --request_id <id>`。
- 新增 `codex-auto-dev code-review --request_id <id>`。
- 新增可替换 reviewer 连接器:
  - `tools/plan-review.sh`
  - `tools/test-review.sh`
  - `tools/design-review.sh`
- 新增 reviewer prompt:
  - `tools/prompts/plan-reviewer.md`
  - `tools/prompts/test-reviewer.md`
  - `tools/prompts/design-reviewer.md`
- 新增结构化输出 schema: `tools/schemas/review-result.schema.json`。
- reviewer 输出必须包含 `reviewer`、`approved`、`decision`、`summary`、`process`、`critical`、`high`、`warning`、`info`。
- reviewer 结果必须写入 change 目录:
  - `reviews/plan-review/plan-reviewer.json`
  - `reviews/code-review/test-reviewer.json`
  - `reviews/code-review/design-reviewer.json`
  - 每个 stage 的 `summary.json`
- 任意 `critical/high` 或 reviewer failure 必须导致 review gate 失败。
- `plan-review` 通过后写入 plan approval。
- `code-review` 必须先确认 plan approval 有效；通过后写入 change-doc approval。
- `upgrade` 需要补齐缺失 reviewer 连接器、prompts 和 schema，但不能覆盖用户已有文件。

## Reviewer 要求

### PlanReviewer

基于需求、现有代码、CodeGraph、spec、plan 和 tasks 审查计划。计划必须可扩展、合理、严格满足需求，并且不能破坏已有功能，除非需求或计划明确说明必要性、影响、兼容和测试。

### TestReviewer

审查实现是否补充足够测试，覆盖新增行为、失败路径、回归路径和目标项目检查。不得删除或弱化已有测试，除非是结构性变更且有说明和替代覆盖。

### DesignReviewer

审查实现是否符合需求和已批准计划，是否无硬编码、无隐私数据、无未授权破坏性变更、无明显 bug，并满足目标项目内部要求。必须检查 plan approval 有效。

## 非目标

- 不在本次实现 `tick` 自动编排。
- 不在本次实现真正并行调度。
- 不绑定 Codex 作为唯一 reviewer 后端。
- 不自动运行 `finish`。

## 验收标准

- 新 workspace 创建默认 reviewer 脚本、prompts 和 schema。
- `plan-review` 在 high finding 时失败，不写 approval。
- `plan-review` 通过时写入 plan approval。
- `code-review` 必须在 plan approval 有效后才能运行。
- `code-review` 中任意 reviewer high finding 会失败，不写 change-doc approval。
- `code-review` 两个 reviewer 都通过时写入 change-doc approval。
