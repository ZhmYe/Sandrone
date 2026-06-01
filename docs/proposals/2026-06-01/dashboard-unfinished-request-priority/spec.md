# Dashboard 未完成需求优先展示

## 背景

当一个项目里已有多个 `finished` request 时，新发现或仍待处理的 request 会被排在列表后方，用户需要滚动才能看到当前要处理的需求。

## 需求

- Dashboard 右侧 request 列表必须优先展示未完成项。
- `finished` request 必须稳定排在未完成 request 后面。
- 同一组内保持 API 返回的原始顺序，避免刷新后列表无意义跳动。
- 只调整前端展示顺序，不改变 `.codex-auto-dev/state/requests.tsv`、dashboard API 数据或状态机语义。

## 非目标

- 不改变项目侧边栏排序。
- 不新增筛选器、搜索或分页。
- 不改变 `finished`、`wait-finish`、`wait-update-pr` 的状态含义。
