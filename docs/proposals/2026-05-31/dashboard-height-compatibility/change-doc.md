# Dashboard Height Compatibility Change Doc

## 摘要

本次优化 dashboard 的高度兼容性和 request/project 卡片的文本布局。页面整体恢复自然文档流，可以超过一屏并由浏览器滚动；局部列表和 artifact 内容区设置最大高度并内部滚动，避免长 Markdown/JSON 无限撑开页面、长路径断行丑陋或 request 列表裁切。

## 实现前后对比

变更前:

- `shell` 和 `sidebar` 依赖 `min-height: 100vh`，没有形成稳定的视口高度约束。
- `artifact-body` 同时设置 `min-height: 380px` 和 `max-height: calc(100vh - 420px)`，在短视口下容易互相冲突。
- 桌面端曾被锁在单屏高度内，artifact 区域拿不到足够空间时会显得过小。
- request 列表高度过紧时会裁掉第三条需求，active/hover 的 `translateY` 也会让边框看起来压到相邻区域。
- 长 workspace path 和 external id 使用普通断行，容易出现难看的局部溢出或贴边。

变更后:

- `body` 和 `shell/main/detail` 恢复自然高度，不再强行限制在一屏。
- `project-list`、`request-list`、`artifact-body` 分别设置最大高度并成为可滚动区域。
- `artifact` 使用 `grid-template-rows: auto minmax(0, 1fr)`，内容区按剩余空间伸缩。
- 移动端取消高度锁定，恢复整页滚动，并确认无横向溢出。
- request 列表高度提高到稳定容纳当前 3 条需求，并保留内部滚动。
- project/request 卡片不再通过位移表达选中态，改用边框和阴影，避免视觉重叠。
- 长路径和 request meta 使用 `overflow-wrap: anywhere` 与两行裁切。

## 关键设计点

- 使用自然文档流承载整体页面高度，只在局部内容区使用最大高度，避免单屏锁定导致阅读区过小。
- 使用 `min-height: 0` 解除 CSS Grid/Flex 子项默认最小内容高度，避免内容把父容器撑开。
- `request-list` 使用更宽松的 `clamp(278px, 34dvh, 360px)`，优先保证 3 条常见 request 不被裁掉。
- artifact 面板改为自然高度并设置 `max-height: 820px`，内容体设置 `max-height: 720px`，不再按一屏视口比例压缩；超过上限后再内部滚动。
- `json-viewer` 使用视口相关最小高度，避免 JSON 视图在小屏强行撑开布局。
- 不改变 dashboard 数据结构和 JS 状态机。

## 验证证据

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`: 通过。
- `cargo test templates_are_external_assets_not_embedded_in_main`: 通过。
- 浏览器验证: 页面可超过一屏自然滚动，artifact 内容区有最大高度并内部滚动，PoorGuy/REQ-0003 可见。
- 浏览器 `390x760`: `body overflow-y = auto`，`scrollWidth = 390`，无横向溢出，PoorGuy 可见。
- 根据用户截图补充修复: request/project 卡片不再使用 active 位移；request list 不再裁掉 REQ-0003；长 path/external id 使用稳定断行和两行裁切；artifact 阅读区域扩大后仍保持内部滚动。

## Review 结果

本次是 dashboard CSS 高度兼容性修复，没有运行自动 reviewer gate；以 dashboard 结构测试、浏览器视口验证、proposal 校验和 diff 空白校验作为交付验证。
