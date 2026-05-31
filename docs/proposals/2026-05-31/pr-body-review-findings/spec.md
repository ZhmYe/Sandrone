# Spec: PR Body Review Findings

## 背景

`finish` 会生成 PR body，并把该 body file 交给 `tools/pr-create.sh`。此前 PR body 包含关联需求、request 文档和 change-doc，但自动评审的具体 finding 主要保存在本地 `reviews/<stage>/details/*.json`。人类在 GitHub PR 页面评审时，很难直接看到 reviewer 的 warning/info、证据、影响和建议修改。

## 目标

- PR body 必须包含 `自动评审意见` section。
- 该 section 必须从最终 review detail JSON 汇总 `critical`、`high`、`warning`、`info`。
- 每条 finding 必须保留 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。
- PlanReviewer、TestReviewer、DesignReviewer 的最终 detail 应分别展示 reviewer summary、decision、recommended next phase 和 detail path。
- 没有自动评审结果时，PR body 必须明确提示这是人工审批或缺少 review 结果，而不是静默省略。

## 非目标

- 不让 `tools/pr-create.sh` 自己解析 review JSON。
- 不把 PR 创建逻辑写死到 GitHub；PR body 仍是平台中立的 markdown 文件。
- 不改变 reviewer gate 的通过/拒绝规则。
- 不在 PR body 中展开历史 review 轮次，只展示最终 summary 对应的 attempt。

## 行为要求

- `finish` 调用 `write_pr_body` 时，必须在 Request 和 Change Doc 之前插入 `自动评审意见`。
- `plan-review` 使用 `reviews/plan-review/summary.json` 的最终 attempt 定位 `PlanReviewer` detail。
- `code-review` 使用 `reviews/code-review/summary.json` 的最终 attempt 定位 `TestReviewer` 和 `DesignReviewer` detail。
- 任何 severity 的 finding 都必须显示；尤其是 approved gate 中仍存在的 warning/info，必须进入 PR 描述，方便人类复核。
- 解析失败或 detail 不存在时不得 panic；PR body 应保留可读提示。

## 验证

- 自定义 code-review detail 中包含 warning/info 时，`finish` 生成的 PR body 包含 reviewer、severity、title、evidence、impact、suggested_change 和 verification。
- 既有 PR 创建、已有 PR 复用、裸 URL connector 兼容行为保持不变。
