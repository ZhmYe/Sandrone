# 规格: Agent Exit Advance Hook

## 背景

外层 `tick` 状态机已经避免了子 Codex 嵌套 reviewer，但仍有一个体验问题: planning agent 或 implementation agent 完成后，需要等下一次 heartbeat 才会进入 review 或下一阶段。15 分钟 heartbeat 会让自动流程不必要地变慢。

## 目标

- 新增 `sandrone advance --request_id <REQ>`，只推进单个 request，不运行 issue update。
- agent wrapper 在 `tools/issue-agent.sh` 退出并写入 exit code 后，立即调用 `advance`。
- `advance` 负责刷新当前 request、提交 gate、运行 reviewer、创建 worktree、派发下一 phase、进入 `waiting-finish` 或 `blocked`。
- 通过 per-request lock 防止 hook 和 heartbeat 同时推进同一个 request。
- 保持最终人工门禁: 不自动 `finish`、commit、push、PR 或 merge。

## 非目标

- 不新增 daemon 或长期驻留进程。
- 不把 reviewer 放回子 agent。
- 不改变 reviewer connector 的可替换性。
- 不改变 finish 阶段的人工确认要求。

## 验收标准

- 一次 `tick` 派发 planning agent 后，agent 退出 hook 能自动推进到 implementation agent。
- implementation agent 退出 hook 能自动运行 code-review 并标记 `waiting-finish`。
- `advance` 使用 request lock；拿不到锁时安全跳过。
- heartbeat `tick` 仍能发现新 issue，并能兜底处理漏掉 hook 的 request。
- 自动推进最多到 `waiting-finish` 或 `blocked`，不会执行 finish。
