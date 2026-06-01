# Dashboard PR Refresh Bypass Timeline Plan

## 目标顺序

1. 调整 timeline DOM，增加 `timeline-track` 作为主线和旁路的定位容器。
2. 用 CSS 让主线保持单行，旁路绝对定位到 5/6 下方。
3. 通过金色 U 型连接线表达旁路从主线分出并回接。
4. 调整 JS 中 branch stage 的 grid column，让两个支线节点落在旁路位置。
5. 更新 dashboard HTML 单测。
6. 用 PoorGuy dashboard 页面做浏览器验证。

## 关键设计

- 使用现有 vanilla HTML/CSS/JS，不新增依赖。
- 保留 timeline 横向滚动能力，避免窄视口挤压文字。
- 旁路节点使用现有 `stage branch` class，只调整位置和连接线。

## 测试

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`
- `cargo fmt --check`
- `git diff --check`
- 浏览器验证 PoorGuy REQ-0002 的 `PR Refresh -> Integration Review` 旁路显示。
