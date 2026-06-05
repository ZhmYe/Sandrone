# Workspace Registry Dashboard Tasks

- [x] 为 workspace registry、dashboard JSON 和 `sdr` 别名补充红灯测试。
- [x] 实现全局 `workspaces.json` 的读写与刷新。
- [x] 将 `new`、`upgrade`、`update`、`list`、`status` 接入 registry。
- [x] 实现 `dashboard --json` 聚合所有已登记 workspace。
- [x] 实现 review attempts 从 detail JSON 分组展示。
- [x] 实现本地 HTTP dashboard 前端。
- [x] 将 request 展示改为纵向列表。
- [x] 接入 Markdown 与 JSON 美化渲染，并保留纯文本 fallback。
- [x] 新增 `sdr` 短命令。
- [x] 更新 README、skill 和 proposal 索引。
- [ ] 运行完整验证命令。

## 后续流程

- 后续可以把大 artifact 内容拆成按需加载接口，减少 dashboard 初始 JSON 体积。
- 后续可以增加 registry discover 命令，从指定根目录扫描旧 workspace。
