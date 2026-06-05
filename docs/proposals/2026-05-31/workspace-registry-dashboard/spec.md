# Workspace Registry Dashboard Spec

## 背景

当前 CLI 已经能在单个 workspace 内自动抓取 request、生成计划包、派发 agent、运行 review gate 和等待 finish。但用户需要一个本地前端总览所有启用外框架的项目，并能按项目、需求和 stage 查看关键产物。

## 目标

- 新增全局 workspace registry，默认路径为 `~/.sandrone/workspaces.json`，支持 `SANDRONE_HOME` 覆盖。
- `new`、`upgrade`、`list` 和 `dashboard` 必须刷新 registry，使旧 workspace 和新 workspace 都能被本地 dashboard 发现。
- 保留 `sandrone` 命令，同时提供短别名 `sdr`。
- 新增 `sandrone dashboard`，启动本地浏览器页面。
- 新增 `sandrone dashboard --json`，输出 dashboard 数据模型，便于测试、前端和后续机器人复用。
- Dashboard 按项目区分 request，并提供 6 段 timeline: `Request -> Plan -> Plan Review -> Implementation -> Code Review -> Finish / PR`。
- 普通 stage 展示一个核心文件；review stage 展示每轮 reviewer detail JSON，不依赖会被覆盖的 `summary.json`。
- request 区域使用纵向列表，便于处理较多需求。
- Markdown 文件使用 `marked` + `DOMPurify` + `highlight.js` 渲染；JSON 文件和 reviewer detail 使用 `jsoneditor` 的只读 view 模式渲染，CDN 不可用时退回纯文本。

## 非目标

- 不引入登录、多用户权限或远端服务。
- 不改自动推进状态机和 reviewer gate 规则。
- 不在主 stage 区域展示 `recovery.md`。
- 不要求前端写入状态；本次 dashboard 只读。

## 数据契约

全局 registry:

```json
{
  "schema_version": 1,
  "workspaces": [
    {
      "key": "/abs/workspace",
      "repo_name": "example",
      "git_url": "https://example/repo.git",
      "workspace_path": "/abs/workspace",
      "target_repo": "/abs/workspace/dev/repo",
      "last_status": "ready",
      "request_count": 1,
      "status_counts": { "discovered": 1 },
      "updated_at": "..."
    }
  ]
}
```

Dashboard API:

- 顶层包含 `projects`。
- project 包含 workspace 元数据、状态计数和 requests。
- request 包含状态、来源、分支、worktree 和 stages。
- review stage 的 `review_attempts` 由 `reviews/<stage>/details/*.json` 按文件名前缀分组生成。

## 风险

- 文件很多时 dashboard JSON 可能较大。本次先对单个 artifact 做长度截断，后续可按需拆分 API。
- `sdr` 作为包装二进制依赖同目录下存在 `sandrone`，符合 `cargo install` 同时安装两个 bin 的场景。
- Dashboard 渲染库来自 CDN，离线环境会回退为纯文本。后续如需完全离线，可把这些资源 vendored 到二进制或 workspace。
