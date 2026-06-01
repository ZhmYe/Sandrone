# Dashboard Integration Review Secondary Timeline Plan

## 目标顺序

1. 调整 dashboard CSS，把 timeline 分为普通 6 段模式和有 Integration Review 的双层模式。
2. 修改前端渲染逻辑，以 `integration-review.review_attempts.length > 0` 作为支线显示条件。
3. 双层模式中，上层隐藏 `Finish / PR`，下层显示 `PR Refresh`、`Integration Review`、`Finish / PR`。
4. 添加斜虚线视觉连接，保持整体色调和现有组件一致。
5. 更新 HTML 单测锚点、README、skill 和 proposal 索引。

## 测试

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`
- `cargo fmt --check`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
- 浏览器验证 PoorGuy dashboard 中有 Integration Review 的 request 使用双层 timeline。
