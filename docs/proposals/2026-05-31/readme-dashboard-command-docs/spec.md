# README Dashboard Command Docs Spec

## 背景

dashboard 和 CLI 命令已经经历多轮增强: 增加 `cad` 短命令、全局 workspace registry、dashboard 页面、三类项目标签、stage 文件展示、局部滚动和旧 workspace 登记。README 需要同步这些实际能力，否则用户会困惑为什么页面为空、标签含义是什么，以及应该用哪些命令刷新 registry 或启动面板。

## 需求

- 更新 README，明确 `cad` 是 `codex-auto-dev` 的短命令别名。
- 说明 dashboard 依赖全局 registry，页面为空时应运行 `cad list`、`cad upgrade` 或 `cad dashboard --json` 刷新登记。
- 说明 dashboard 当前支持的项目侧边栏、三类标签、request 列表、timeline、Markdown/JSON/review detail 展示和局部滚动行为。
- 更新命令参考中 dashboard 的 host/port、`--port 0`、json、HTTP 端点和旧 workspace 登记说明。

## 非目标

- 不修改 CLI 行为。
- 不修改 dashboard UI。
- 不改变 skill 或 connector 契约。
