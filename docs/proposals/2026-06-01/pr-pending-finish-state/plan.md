# PR Pending Finish State Plan

## 目标顺序

1. 增加 `wait-finish` 状态和进度 rank，让自动 tick 把它视为终止等待态。
2. 调整 `finish`: 首次 PR 创建/复用后进入 `wait-finish`；PR 创建失败则回到 `wait-update-pr`。
3. 新增 `pr-status` 命令，复用 `tools/pr-status.sh`，确认 `merged` 后才写 `finished`。
4. 支持 legacy 修正: `finished` 状态下如果 PR 脚本返回 `open`，回写为 `wait-finish`；如果返回 `missing` 或 `closed`，回写为 `wait-update-pr`。
5. 更新 dashboard: request pill 显示 `wait-finish`，项目汇总增加 `PR 待合并`，finish 计数只统计 `finished`。
6. 更新 README、skill 和 proposal 索引。
7. 补充测试覆盖 PR pending、merged 确认、legacy 修正入口和 pr-refresh 兼容。

## 关键设计

- `finish` 表示“交付到 PR”，不再表示“需求合并完成”。
- `pr-status` 是只读观察命令，所有平台差异仍在 `tools/pr-status.sh` 中实现。
- `wait-finish` 和 `wait-update-pr` 都是自动 tick 的终止等待态，但语义不同:
  - `wait-update-pr`: code-review 通过，等待人类决定是否交付 PR。
  - `wait-finish`: PR 已创建或复用，等待外部平台合并。
- `finished` 只能由 PR status 脚本确认 merged 后写入。

## 测试

- `finish_requires_change_doc_approval_then_commits_and_pushes_request_branch`
- `finish_reports_existing_pr_from_pr_connector`
- `pr_refresh_conflict_uses_rebase_agent_and_integration_review`
- `dashboard_html_uses_list_requests_and_rich_artifact_renderers`
- 全量 `cargo test`
