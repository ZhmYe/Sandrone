# PR Refresh Integration Review Plan

## 目标顺序

1. 增加测试，锁定默认模板、帮助命令、finish 无新改动刷新、冲突 rebase 支线。
2. 增加 CLI 命令 `pr-refresh` 和 `integration-review`。
3. 增加 RebaseAgent agent phase 与状态机状态。
4. 增加 IntegrationReviewer review gate，复用严格 JSON schema。
5. 扩展默认 assets/templates 和 upgrade examples。
6. 更新 dashboard 数据模型和前端 timeline，让 PR refresh/rebase 作为 `Finish / PR` 后的可视化支线展示。
7. 更新 README、skill 和 proposal 索引。
8. 运行格式化、测试、clippy、proposal 校验和 diff 检查。

## 关键设计

- `pr-refresh` 是 finish 后支线入口，不属于 `tick` 的普通 issue 扫描。
- Clean rebase 直接进入 `integration-review-submitted`，通过后写入 change-doc approval 并回到 `wait-update-pr`。
- Conflict rebase 进入 `rebase-agent-running`，由 RebaseAgent 解决冲突；agent hook 调用 `advance` 后运行 IntegrationReviewer。
- IntegrationReviewer 通过时重新批准 `change-doc`，使 `finish` 可以安全更新 PR。
- IntegrationReviewer 拒绝时进入 `integration-review-rejected`，下一次 `advance/tick` 会继续派发 RebaseAgent；超过 max attempts 后 block。
- Dashboard 保留 6 段主线，同时在 `Finish / PR` 后展示 `PR Refresh -> Integration Review` 支线；支线 stage 读取 `change-doc.md` 和 `reviews/integration-review/details/*.json`，方便观察 rebase 冲突原因、解决方式和轻量集成评审结果。

## 测试

- `help_lists_state_and_validation_commands`
- `templates_are_external_assets_not_embedded_in_main`
- `new_name_creates_framework_and_empty_target_repo_only`
- `finish_reports_existing_pr_from_pr_connector`
- `dashboard_html_uses_list_requests_and_rich_artifact_renderers`
- `dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`
- `pr_refresh_conflict_uses_rebase_agent_and_integration_review`
- 全量 `cargo test`
