# 变更文档: Strict Reviewer Gates

## 摘要

本次变更新增 `plan-review` 和 `code-review` 两个硬门禁命令，并生成三个可替换 reviewer connector。自动化审批不再依赖可见 session，而是依赖结构化 reviewer JSON。任意 `critical/high` 都会阻断流程。

## 实现前后对比

- 实现前: 自动化只能通过人工或外部脚本调用 `approve`，缺少标准化 plan/code review gate。
- 实现后: `PlanReviewer` 通过后才自动 approval plan；`TestReviewer` 和 `DesignReviewer` 都通过后才自动 approval change-doc。review 结果写入 change 目录，后续可审计。

## 关键设计点

### 可替换 Reviewer Connector

默认脚本位于 `tools/plan-review.sh`、`tools/test-review.sh` 和 `tools/design-review.sh`，内部使用 `codex exec` 和 `tools/schemas/review-result.schema.json`。脚本只要求 stdout 输出结构化 JSON，因此可以替换为 Claude Code、OpenAI API 或公司内部 LLM。

### 结构化阻断

CLI 会把每个 reviewer 输出写入 `docs/changes/<name>/reviews/<stage>/`。只有 `approved=true` 且 `critical/high` 均为空时，该 reviewer 才算通过。脚本失败、输出为空或缺少 `approved` 都会被转换成 blocking critical JSON。

### Approval 联动

`plan-review` 通过后写入 `approvals/plan.approval.json`。`code-review` 先检查 plan approval 有效，再运行 TestReviewer 和 DesignReviewer；两者都通过后写入 `approvals/change-doc.approval.json`。

## 变更范围摘要

主要改动集中在 CLI 命令、默认 tools、review prompts/schema、runtime workspace upgrade、README、skill 和集成测试。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`

## 风险与后续

- 本次没有实现 `tick` 编排；后续可以让 tick 按 update -> plan agent -> plan-review -> start -> implementation agent -> code-review 的顺序运行。
- 当前 JSON 检查足够用于门禁，但未来可以引入正式 JSON parser 或独立 validator。
