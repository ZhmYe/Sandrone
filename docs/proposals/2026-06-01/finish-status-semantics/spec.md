# Finish 状态语义收敛

## 背景

PR 交付链路里原先的 `waiting-finish`、`pr-pending` 和 `finished` 语义容易混在一起。尤其是在 code-review 通过、IntegrationReview 通过、PR 已创建、PR 已合并这些节点之间，CLI、dashboard 和外部 workspace 的旧状态需要有稳定的迁移规则。

## 需求

- `wait-update-pr` 表示代码评审或集成评审已通过，PR 需要创建、更新或重新创建。
- `wait-finish` 表示 PR 已创建或已更新，正在等待目标平台合并。
- `finished` 只能表示通过 `tools/pr-status.sh` 确认 PR 已合并。
- legacy `waiting-finish` 必须兼容为 `wait-update-pr`。
- legacy `pr-pending` 必须兼容为 `wait-finish`。
- code-review 和 integration-review 通过后应直接落到 `wait-update-pr`，不依赖下一次 tick 再补状态。
- `tools/pr-status.sh` 返回 `open` 时应保持或修正为 `wait-finish`；返回 `missing` 或 `closed` 时应回到 `wait-update-pr`。
- Dashboard 的 Finish / PR 节点只有在 `finished` 时为完成态；`wait-update-pr` 和 `wait-finish` 都是未完成的等待态或待操作态。

## 非目标

- 不改变 PR 创建脚本、PR 状态脚本的可替换接口。
- 不自动 merge PR。
- 不移除历史 proposal 中已记录的旧术语，但当前 README、skill 和最新 proposal 必须使用新语义。
