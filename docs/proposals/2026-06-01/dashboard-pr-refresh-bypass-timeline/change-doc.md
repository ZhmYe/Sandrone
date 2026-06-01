# Dashboard PR Refresh Bypass Timeline Change Doc

## 摘要

本次把 dashboard 的 PR refresh 支线从完整第二行改成 5/6 附近的旁路。视觉上主流程仍是一条线，`PR Refresh` 和 `Integration Review` 作为 PR 后的集成刷新支路挂在右侧下方。

## 实现前后对比

变更前:

- PR 支线占据主线下方整行。
- 连接线像第二条平行流水线，容易误解为独立流程。

变更后:

- 新增 `timeline-track` 定位容器。
- 主线保持 6 个阶段单行。
- 支线节点绝对定位在主线 5/6 下方，使用金色 U 型连接线表达旁路。
- 点击 stage、artifact 渲染和 review detail 展示不变。

## 验证证据

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`: 通过。
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- 浏览器验证 `http://127.0.0.1:47222/` 中 PoorGuy REQ-0002 支线显示为旁路。
