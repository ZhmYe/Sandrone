# Dashboard

`sdr dashboard` 启动本地 HTTP 页面，用来观察本机所有已登记 workspace 的需求进展。

```bash
sdr dashboard
sdr dashboard --host 127.0.0.1 --port 47220
sdr dashboard --port 0
sdr dashboard --json
```

## 数据来源

Dashboard 读取全局：

```text
~/.sandrone/workspaces.json
```

`new`、`upgrade`、`list`、`dashboard` 会刷新 registry。旧 workspace 没出现在页面里时，进入该 workspace 执行：

```bash
sdr list
```

或：

```bash
sdr upgrade
```

## 页面结构

- 左侧：项目侧边栏，按 workspace 分组。
- 右侧上方：当前项目摘要和父 request 列表。
- 右侧下方：request 详情。
- 详情 tab：`需求分析 | Slice 1 | Slice 2 ... | PR`。

主列表只展示父 request，不把 slice 当作独立需求刷在列表里。未完成 request 优先显示，`finished` 稳定排在后面。

## 项目标签

左侧项目只显示三类标签：

| 标签 | 含义 |
| --- | --- |
| `blocked` | 有 request blocked。 |
| `pending` | 非 blocked、非 finished 的所有状态，包括 `wait-update-pr` 和 `wait-finish`。 |
| `finish` | 已确认 finished 的 request。 |

## Stage 展示

`需求分析` tab 展示：

- `<REQ> request.md`
- `<REQ> decomposition.md`
- Decomposition Review detail

每个 slice tab 展示：

```text
Plan -> Implementation
```

Plan Review 折叠在 Plan 阶段的 `Review 结果` tab；Code Review 折叠在 Implementation 阶段的 `Review 结果` tab。slice timeline 不再单独展示 request 节点。

`PR` tab 展示父 request 的交付状态：

```text
PR Delivery
```

如果存在 PR refresh、rebase 冲突或 Integration Review 记录，`PR` tab 会追加展示：

```text
PR Delivery -> PR Refresh -> Integration Review
```

Integration Review 通过后，状态回到 `wait-update-pr`，表示需要再次运行 `finish` 推送 rebase 后的分支。

## Artifact 渲染

- Markdown 使用 `marked`、`DOMPurify` 和 `highlight.js`。
- JSON 和 reviewer detail 使用 `jsoneditor` 只读 view。
- CDN 不可用时回退到纯文本展示。
- request 列表、项目列表、artifact 阅读区都有最大高度和内部滚动；整个页面也允许自然超过一屏。

Review detail 读取不可变文件：

```text
reviews/decomposition-review/details/*.json
reviews/plan-review/details/*.json
reviews/code-review/details/*.json
reviews/integration-review/details/*.json
```

多轮 review 按 `001-*`、`002-*` 分组展示。`summary.json` 只代表最新汇总，可能被覆盖，因此不作为详情展示的唯一来源。

## API

| 路径 | 说明 |
| --- | --- |
| `/` | Dashboard 页面。 |
| `/api/dashboard` | 与 `sdr dashboard --json` 等价的数据模型。 |
| `/api/health` | 健康检查。 |
