# PR Refresh Integration Review Change Doc

## 摘要

本次增加 finish 后的 PR refresh/rebase 集成支线。框架现在可以在 PR 已创建后重新 fetch base、尝试 rebase、遇到冲突时派发 RebaseAgent，解决后运行轻量但严格的 IntegrationReviewer。IntegrationReviewer 通过后重新批准 change-doc，request 回到 `wait-update-pr`，用户可以再次运行 `finish` 更新同一 PR 分支。

## 实现前后对比

变更前:

- `finish` 只覆盖首次 commit/push/PR。
- PR 创建后如果 master 前进或发生冲突，需要人工处理，框架没有专门状态、agent 或 review gate。
- 没有新文件改动时再次 `finish` 会失败。

变更后:

- 新增 `pr-refresh --request_id REQ-0001` 处理 PR refresh 支线。
- Clean rebase 直接运行 IntegrationReviewer。
- Conflict rebase 派发 RebaseAgent；RebaseAgent 必须保留 base/master 新代码和 request 分支语义。
- IntegrationReviewer 审查冲突标记、base/master 保留、需求语义、测试证据和 change-doc 记录。
- `finish` 支持无新改动刷新 PR，必要时 force-with-lease 推送 request 分支。
- Dashboard 在 `Finish / PR` 后展示 `PR Refresh -> Integration Review` 支线，可以直接查看 `change-doc.md` 的集成刷新记录和 IntegrationReviewer 的每轮 detail JSON。

## 关键设计点

### RebaseAgent 与 implementation agent 分离

RebaseAgent 只做集成刷新，不扩大需求范围，不处理新功能。它的 prompt 明确禁止为了自己分支删除 base/master 新代码，并要求逐项记录冲突原因、解决方式、两边保留证明和验证结果。

### IntegrationReviewer 是轻量集成门禁

IntegrationReviewer 不替代首次 code-review。它只审查 rebase 后是否安全，重点看冲突是否干净、base/master 是否被保留、原 approved plan 语义是否仍成立、是否有测试证据。

### finish 二次刷新

当 worktree 没有新文件改动时，`finish` 不再直接失败，而是重新生成 PR body、push request 分支并复用已有 PR。push 非快进失败时使用 `--force-with-lease`，避免粗暴覆盖远端未知更新。

### Dashboard 支线观察

Dashboard 继续保留 `Request -> Plan -> Plan Review -> Implementation -> Code Review -> Finish / PR` 主线。PR 已创建后如果进入 refresh/rebase 流程，页面会从 `Finish / PR` 分出 `PR Refresh -> Integration Review` 支线。`PR Refresh` 读取 `change-doc.md` 中的集成刷新记录；`Integration Review` 读取 `reviews/integration-review/details/*.json`，多轮 review 按 attempt 展示，便于人类 reviewer 追溯冲突处理和集成门禁结果。

## 验证证据

- `cargo test pr_refresh_conflict_uses_rebase_agent_and_integration_review -- --nocapture`: 通过。
- `cargo test new_name_creates_framework_and_empty_target_repo_only -- --nocapture`: 通过。
- `cargo test finish_reports_existing_pr_from_pr_connector -- --nocapture`: 通过。
- `cargo test help_lists_state_and_validation_commands -- --nocapture`: 通过。
- `cargo test templates_are_external_assets_not_embedded_in_main -- --nocapture`: 通过。
- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers -- --nocapture`: 通过。
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts -- --nocapture`: 通过。
- `cargo test`: 通过，43 个集成测试和 1 个单元测试通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，37 个 proposal 通过校验。
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。

## Review 结果

本次为框架自身变更，使用 Rust 测试、clippy、proposal 校验和 diff 检查作为交付验证。新增 IntegrationReviewer prompt 和 RebaseAgent prompt 已通过模板生成测试覆盖。
