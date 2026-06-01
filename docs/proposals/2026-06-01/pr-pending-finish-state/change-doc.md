# PR Pending Finish State Change Doc

## 摘要

本次把 PR 已提交和需求已完成拆成两个状态。`finish` 创建或复用 PR 后进入 `wait-finish`；只有 `tools/pr-status.sh` 返回 `merged`，`pr-status` 或二次 `finish` 才会把 request 标记为 `finished`。

## 实现前后对比

变更前:

- `finish` 一旦 PR 创建/复用成功就写 `finished`。
- dashboard 的 finish 计数包含尚未合并的 PR。
- legacy 状态无法通过脚本校正。

变更后:

- `finish` 首次交付成功写 `wait-finish`。
- `finish` 在 `wait-finish` 或 legacy `finished` 状态下只运行 PR 合并确认，不再 commit/push。
- `pr-status --request_id <REQ>` 可显式检查 PR 状态；返回 `merged` 才写 `finished`。
- 如果脚本返回 `open`，状态保持或修正为 `wait-finish`；如果脚本返回 `missing` 或 `closed`，状态回到 `wait-update-pr`，等待重新创建或更新 PR。
- dashboard 增加 `PR 待合并` 统计，`finish` 仍只统计真正 `finished`。

## 关键设计点

### 状态语义

`wait-update-pr` 表示实现和 code-review 已完成但尚未交付 PR。`wait-finish` 表示 PR 已交付但尚未合并。`finished` 只能表示 PR 已合并。

### 平台适配

Rust 代码只解析 `tools/pr-status.sh` 输出的 `status<TAB>url<TAB>detail`。GitHub、GitLab 或内部系统的查询方式都保留在可替换脚本里。

### Legacy 修正

为了解决旧版本把 open PR 标记为 `finished` 的问题，`finish` 和 `pr-status` 都允许在 `finished` 状态下重新检查 PR。如果脚本返回 `open`，会回写 `wait-finish`；如果脚本返回 `missing` 或 `closed`，会回写 `wait-update-pr`。

## 验证证据

- `cargo test finish_requires_change_doc_approval_then_commits_and_pushes_request_branch`: 通过。
- `cargo test finish_reports_existing_pr_from_pr_connector`: 通过。
- `cargo test pr_refresh_conflict_uses_rebase_agent_and_integration_review`: 通过。
- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`: 通过。
