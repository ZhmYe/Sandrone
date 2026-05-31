# Dashboard Height Compatibility Spec

## 背景

dashboard 页面在较矮桌面窗口或移动视口中会出现高度不兼容: 桌面端外层页面和内部 artifact 区域同时参与滚动，`artifact-body` 的固定 `min-height` 与 `max-height: calc(...)` 容易让内容区被挤压或撑出视口。

## 需求

- 桌面端 dashboard 应稳定占满当前视口高度。
- 左侧项目列表、需求列表和 artifact 内容区应各自内部滚动，避免整页和面板同时抢滚动。
- 移动端应恢复自然整页滚动，不能产生横向溢出。
- 不改变 dashboard 数据 API、交互逻辑、stage 结构或视觉语义。

## 非目标

- 不重做 dashboard 信息架构。
- 不引入新的前端依赖。
- 不改变 registry、request 或 review 数据生成逻辑。
