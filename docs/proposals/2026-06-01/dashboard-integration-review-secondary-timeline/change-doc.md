# Dashboard Integration Review Secondary Timeline Change Doc

## 摘要

本次优化 dashboard 的 PR 集成刷新展示方式。没有 Integration Review 详情时，页面只显示普通主线；有 Integration Review 详情时，页面切换为双层 timeline，把 PR Refresh、Integration Review 和 Finish / PR 放到下方居中支线。

## 实现前后对比

变更前:

- PR Refresh 和 Integration Review 总是作为 branch stage 参与布局。
- 支线挤在主线右侧，视觉上容易拥挤。

变更后:

- `hasIntegrationFlow` 只根据 `integration-review.review_attempts` 判断是否显示支线。
- 无 Integration Review 时隐藏 PR Refresh 与 Integration Review。
- 有 Integration Review 时，上层只到 Code Review，下层居中显示 PR Refresh、Integration Review、Finish / PR。
- 下层顺序为 `PR Refresh -> Integration Review -> Finish / PR`，`Finish / PR` 是最后交付节点。
- 虚线连接从右上 Code Review 接到下层左侧 PR Refresh，改为中性灰 SVG 贝塞尔曲线；位置由 JS 根据两个节点的真实 `getBoundingClientRect` 计算，控制点限制在 Code Review 与 PR Refresh 之间，避免不同窗口宽度下明显歪斜、横穿主线文字或插到 `Finish / PR` 下方。
- 下层支线轨道未完成节点使用灰色语义；只有完成节点显示绿色，进行中/待操作节点显示蓝色，warning/amber 不用于流程线装饰。
- Integration Review 通过后 request 回到 `wait-update-pr`，Dashboard 会把 `Finish / PR` 显示为当前待操作节点，直到再次运行 `finish` 推送 rebase 后分支并刷新 PR。
- `PR Refresh` stage 不再展示整份 `change-doc.md`。如果存在 `pr-conflicts/attempts/*.md`，它会直接串联展示真实冲突 attempt；如果没有冲突 attempt，才从 `change-doc.md` 抽取 `PR 集成刷新记录` / `PR 冲突记录` 章节作为 clean rebase 或普通刷新证据。

## 验证证据

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`: 通过。
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`: 通过，覆盖 Integration Review 后 `Finish / PR` 为当前待操作节点，并覆盖 `PR Refresh` 优先展示 `pr-conflicts/attempts` 冲突记录。
- `python3 scripts/validate_proposals.py`: 通过，validated 42 proposal(s)。
- `git diff --check`: 通过。
- `cargo build`: 通过，并已重启 `http://127.0.0.1:47222/` 使用新版 dashboard。
