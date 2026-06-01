# PR Conflict Attempt Records Spec

## 背景

`pr-refresh` 可以在 PR 创建后多次运行。base/master 多次前进时，真正的 rebase 冲突也可能不止一次。此前集成刷新记录主要写入 `change-doc.md`，能说明流程发生过，但不适合长期保留每次冲突的原始诊断。

## 需求

- 只有真实 rebase 冲突发生时，才创建独立冲突记录。
- 冲突记录必须按 request 递增编号，支持同一个 PR 多次冲突。
- 冲突记录必须包含 request ID、时间、base branch/ref、rebase 前 HEAD、PR 状态脚本输出、冲突诊断和处理约束。
- `change-doc.md` 必须追加对应的 `PR 冲突记录`，并链接到独立冲突记录文件。
- clean rebase、merged skip、普通 continue 不生成冲突 attempt 文件。

## 非目标

- 不增加 dashboard attempt 列表。
- 不改变 IntegrationReviewer 的 detail JSON 展示方式。
- 不改变 finish、PR 创建或 merge 状态判断。
