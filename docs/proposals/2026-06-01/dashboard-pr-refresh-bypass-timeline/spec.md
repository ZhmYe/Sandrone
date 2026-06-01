# Dashboard PR Refresh Bypass Timeline Spec

## 背景

Dashboard 已支持 `PR Refresh -> Integration Review` 支线，但初版把支线画成主线下方的一整条第二行，视觉上像另一个并列流水线。用户期望它更像从 `Code Review -> Finish / PR` 之间分出的旁路。

## 需求

- 主线仍保持 `Request -> Plan -> Plan Review -> Implementation -> Code Review -> Finish / PR`。
- `PR Refresh` 和 `Integration Review` 必须显示在主线右侧下方的旁路中。
- 旁路应从主线 5/6 附近下探，并用金色连接线表达“PR 后集成刷新”。
- 点击支线节点仍只展示对应 stage 文件。
- 不引入新的前端依赖。

## 非目标

- 不改变 dashboard JSON 结构。
- 不改变 request 状态机或 reviewer gate。
- 不改动 Markdown/JSON artifact 渲染逻辑。
