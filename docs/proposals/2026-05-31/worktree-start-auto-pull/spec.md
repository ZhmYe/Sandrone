# Spec: Worktree Start Auto Pull

## 背景

`sandrone plan` 会在创建计划前检测远端是否领先，但 plan 创建完成后到 implementation worktree 创建之间，目标仓库仍可能有新提交。如果 `start` 只 `fetch` 不 `pull`，新 worktree 可能基于过期的本地分支创建，导致实现、review 和后续 PR 都落在旧基线上。

## 目标

- 创建新的 request worktree 前，自动检测目标仓库是否可以同步远端。
- 对非空且有 remote 的 `dev/repo` 自动运行 `git pull --ff-only`。
- 如果 fast-forward 成功，基于更新后的 `dev/repo` 创建 worktree。
- 如果 pull 失败、分叉或冲突，必须 block request，不得创建 worktree。
- 手动 `start` 和自动 implementation 派发都必须走同一逻辑。

## 非目标

- 不自动 merge。
- 不自动 rebase。
- 不在已经存在的 request worktree 上强制同步。
- 不删除已有 worktree 或用户改动。

## 行为要求

- `dev/repo` 为空时跳过 pull。
- `dev/repo` 没有 remote 时跳过 pull。
- `git pull --ff-only` 成功且 HEAD 更新时，stdout 提示已更新。
- `git pull --ff-only` 成功但 HEAD 未变时，stdout 提示已是最新。
- `git pull --ff-only` 失败时，`start` 返回失败，request 状态写为 `blocked`，`status.json` 和 `recovery.md` 记录失败原因。
- pull 失败时不得创建 `dev/worktrees/<REQ>`。

## 验证

- plan approval 之后远端新增提交，`start` 会自动 pull，`dev/repo` 和新 worktree 都包含远端新增文件。
- 本地和远端分叉时，`start` 失败并 block request，不创建 worktree。
