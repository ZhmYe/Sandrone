# PR Pending Finish State Spec

## 背景

`finish` 之前在创建或复用 PR 后立即把 request 标记为 `finished`。这会混淆两个不同事实:

- 代码已经提交到 request 分支并创建了 PR。
- PR 已经被合入 base/master，需求真正完成。

对于已创建但尚未合并的 PR，框架需要保留一个可观察、可恢复的中间状态，避免 dashboard 和 `list` 把未合并工作误报为完成。

## 需求

- `finish` 首次交付成功创建或复用 PR 后，request 状态必须是 `wait-finish`。
- 如果 PR connector 失败或无法创建 PR，request 不能进入 `wait-finish` 或 `finished`，应保持 `wait-update-pr`，允许操作者修复 connector 后重试。
- 新增 `pr-status --request_id <REQ>`，通过 `tools/pr-status.sh` 观察 PR 状态。
- `finish` 在 `wait-finish` 或 legacy `finished` 状态下不再 commit/push，而是执行同 `pr-status` 一样的合并确认。
- 只有 `tools/pr-status.sh` 返回 `merged` 时，request 才能标记为 `finished`。
- 如果 legacy `finished` request 实际仍是 open PR，运行 `pr-status` 或二次 `finish` 应把它修正为 `wait-finish`；如果 PR 缺失或已关闭，应修正为 `wait-update-pr`。
- dashboard 必须把 `wait-finish` 作为已交付但未合并的状态展示，`finish` 统计只包含真正 `finished`。

## 非目标

- 不自动 merge PR。
- 不新增平台特定 PR API 到 Rust 代码里，仍由 `tools/pr-status.sh` 适配 GitHub、GitLab 或内部平台。
- 不让 `tick` 自动运行 `finish`、`pr-status` 或 merge 检查。
