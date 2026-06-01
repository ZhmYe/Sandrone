# Dashboard 未完成需求优先展示变更文档

## 摘要

Dashboard 现在会把未完成 request 排在列表前面，`finished` request 排到后面。同一组内保持原始顺序，避免刷新时产生无意义跳动。

## 实现前

- Request 列表直接使用 dashboard API 返回顺序。
- 当 REQ-0001、REQ-0002、REQ-0003 都完成，而 REQ-0004 仍是 `discovered` 时，REQ-0004 会显示在完成项后面。

## 实现后

- 前端新增 `orderedRequests(project)` 和 `requestSortRank(request)`。
- `currentRequest()` 与 `renderRequests()` 都读取排序后的 request 列表。
- `status === "finished"` 的 request 排在后面，其他状态排在前面。
- 后端 API 和状态文件完全不变。

## 验证

- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`: 通过。
- `cargo build`: 通过。
- `git diff --check`: 通过。

## 风险

当前选择的是展示层排序，不改变 API。因此如果未来需要机器人按 dashboard 顺序处理 request，应该使用状态机选择逻辑而不是解析页面顺序。
