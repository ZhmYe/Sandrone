# Dashboard Height Compatibility Plan

## 目标与依赖顺序

1. 将桌面布局根容器改为 `100dvh` 高度模型。
2. 给 sidebar、main、detail、artifact 建立 `min-height: 0` 和明确内部滚动边界。
3. 移除 artifact 内容区固定 `min-height + max-height calc` 的冲突，改为 grid 剩余空间。
4. 给移动端恢复 `body` 自然滚动并避免横向溢出。
5. 运行 dashboard 结构测试和浏览器视口验证。

## 设计规则

- 桌面端: 页面整体不滚动，具体列表和文件内容滚动。
- 移动端: 页面整体滚动，局部内容不强行锁死高度。
- 不改 DOM 数据结构，避免影响现有 JS 渲染和测试。

## 验证

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`
- `cargo test templates_are_external_assets_not_embedded_in_main`
- 浏览器验证 `1280x720` 和 `390x760` 视口。
- `python3 scripts/validate_proposals.py`
- `git diff --check`
