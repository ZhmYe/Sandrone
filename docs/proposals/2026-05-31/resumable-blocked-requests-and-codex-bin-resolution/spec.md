# Spec: Resumable Blocked Requests And Codex Bin Resolution

## 背景

自动流程里出现两个恢复问题:

- 从普通终端运行默认 `tools/issue-agent.sh` 或 reviewer connector 时，`codex` 可能不在 `PATH`，导致 agent/reviewer backend 不可用。
- `codex-auto-dev resume --request_id <REQ>` 只打印恢复包路径，没有把 `blocked` request 改回可派发状态。下一次 `tick` 会把它当作 terminal status 跳过。

## 目标

- 默认 agent/reviewer connector 必须能通过配置解析 Codex CLI，不写死用户机器上的 app 路径。
- 默认解析顺序必须是: `CODEX_AUTO_DEV_CODEX_BIN`、当前 `PATH`、`CODEX_AUTO_DEV_CODEX_APP` bundle 内候选 CLI。
- `resume` 对 `blocked` request 必须写回中央索引和 runtime `status.json`，让后续 `tick --request_id <REQ>` 能继续派发。
- 恢复后的 phase 必须由 approval 状态决定: plan approval 不存在或失效时回到 `planning`；plan approval 有效时回到 `in-progress` 并继续 implementation。

## 非目标

- 不把 `/Applications/Codex.app` 或其他本机路径写进默认脚本。
- 不自动修改用户 shell profile。
- 不自动绕过 reviewer gate。
- 不删除 recovery 文档或历史 review 记录。

## 行为要求

- `tools/issue-agent.sh` 和默认 reviewer scripts 必须包含 `resolve_codex_bin`。
- 如果 `CODEX_AUTO_DEV_CODEX_BIN` 指向可执行文件或 PATH 命令，使用它。
- 如果当前 `PATH` 能找到 `codex`，使用它。
- 如果设置了 `CODEX_AUTO_DEV_CODEX_APP`，只在该 bundle 内检查相对候选位置。
- 如果无法解析 Codex CLI，agent connector 非 0 退出，reviewer connector 返回 `gate_unavailable=true`。
- `resume` 对 `blocked` request 必须输出 `resumed status`，并把 `requests.tsv` 与 `status.json` 从 `blocked` 改为可派发状态。
- `tick --request_id <REQ>` 必须能派发刚 resume 的 request。

## 验证

- 新 workspace 生成的默认 issue-agent 和 reviewer scripts 包含 `CODEX_AUTO_DEV_CODEX_BIN`、`CODEX_AUTO_DEV_CODEX_APP` 和 `resolve_codex_bin`。
- 默认 scripts 不包含 `/Applications/Codex.app`。
- blocked request 运行 `resume` 后，状态变为 `planning`，`tick --request_id` 能派发 planning agent。
