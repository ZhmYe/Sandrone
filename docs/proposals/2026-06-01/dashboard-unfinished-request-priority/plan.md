# Dashboard 未完成需求优先展示计划

## 目标

1. 在 dashboard 前端新增 request 展示排序函数。
2. `currentRequest()` 和 `renderRequests()` 都使用同一排序结果，避免列表与详情不一致。
3. 保持 API 数据和后端状态文件不变。
4. 更新 README、skill 和 dashboard HTML 测试断言。

## 实现方式

- 新增 `orderedRequests(project)`，先把 request 映射为 `{ request, index }`。
- `requestSortRank()` 将 `finished` 排到后面，其他状态排前面。
- 排名相同则使用原始 index，保持稳定顺序。

## 测试计划

- `cargo fmt --check`
- `cargo test dashboard_html_uses_list_requests_and_rich_artifact_renderers`
- `cargo build`
- `git diff --check`
