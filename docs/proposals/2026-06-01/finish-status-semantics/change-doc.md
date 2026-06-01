# Finish 状态语义收敛变更文档

## 需求

明确区分 PR 待创建/更新、PR 待合并、PR 已合并三个状态，并能兼容旧 workspace 中已有的 `waiting-finish` 和 `pr-pending`。

## 实现前

- code-review 通过后可能先停在 `change-doc-approved`，再由 tick 补到等待交付状态。
- `waiting-finish` 和 `pr-pending` 的旧语义容易让 dashboard、list 和 status 展示不一致。
- `finished` 可能被旧版本用于表示 PR 已创建，而不是 PR 已合并。
- `closed` PR 会被当作 PR 待合并，语义不够准确。

## 实现后

- `canonical_status()` 把 `waiting-finish` 映射为 `wait-update-pr`，把 `pr-pending` 映射为 `wait-finish`。
- `load_requests()`、`status_progress_rank()`、`is_terminal_status()` 和 `sync_request_from_status_json()` 都使用 canonical 语义。
- code-review / integration-review 通过后直接调用 `mark_wait_update_pr_by_id()`，写入 `status.json`、events 和 session registry。
- `finish` 创建或复用 PR 成功后进入 `wait-finish`；失败则保持或回到 `wait-update-pr`。
- `pr-status` 返回 `merged` 才进入 `finished`；返回 `open` 进入 `wait-finish`；返回 `missing` 或 `closed` 回到 `wait-update-pr`。
- Dashboard 的 Finish / PR 节点只有 `finished` 才是完成态，`wait-update-pr` 和 `wait-finish` 都不会被统计为 finish。

## 验证

- `cargo test --test cli_flow`: 通过。

## 后续

- 在 PoorGuy 这类旧 workspace 上运行新版二进制进行状态迁移和真实 PR 推进验证。
