# Workspace Registry Dashboard Plan

## 实现步骤

1. Registry
   - 增加 `WorkspaceRecord`。
   - 增加 `CODEX_AUTO_DEV_HOME` 和默认 `~/.codex-auto-dev/workspaces.json` 解析。
   - 实现 registry load/save/upsert/refresh。
   - 在 `new`、`upgrade`、`update`、`list`、`status` 中刷新当前 workspace。

2. Dashboard 数据
   - 新增 `dashboard --json`。
   - 读取全局 registry，刷新每个仍存在的 workspace。
   - 从 `requests.tsv`、`status.json` 和 stage 文件生成项目/需求/stage JSON。
   - review stage 只读取 `reviews/<stage>/details/*.json`，按 attempt 分组。

3. Dashboard 前端
   - 新增 `dashboard` HTTP 服务，默认 `127.0.0.1:47217`。
   - 提供 `/` 静态页面和 `/api/dashboard` 数据接口。
   - UI 左侧展示项目列表与状态气泡，右侧展示纵向 request 列表、圆点 timeline 和选中 stage 内容。
   - Markdown 用 `marked`、`DOMPurify` 和 `highlight.js` 呈现；JSON/review detail 用 `jsoneditor` 呈现。
   - review stage 提供多轮 attempt 和 reviewer detail 交互。

4. CLI 别名
   - 新增 `cad` bin，作为 `codex-auto-dev` 包装器。

5. 文档与验证
   - 更新 README 和 skill。
   - 增加集成测试覆盖 registry、dashboard JSON 和 `cad`。
   - 运行 format、check、clippy、test、proposal 校验和 diff check。

## 兼容性

- 旧 workspace 运行 `codex-auto-dev upgrade` 后会写入全局 registry。
- `codex-auto-dev list` 仍只显示当前 workspace 的 request，不变成全局列表。
- `dashboard` 可以从任意目录运行，只依赖全局 registry。
