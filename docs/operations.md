# 运行、交付与恢复

## 自动化运行

最简单的周期任务是定时执行：

```bash
cd /path/to/workspace
sdr tick
```

`tick` 不会运行 `finish`，也不会 merge。它只扫描、派发、review、修复和推进到 `wait-update-pr`。

建议第一次先手动运行：

```bash
sdr doctor
sdr tick
sdr list
```

确认环境稳定后，再交给 Codex heartbeat、cron、LaunchAgent 或内部调度器。

## Finish 与 PR

自动流程通过 code-review 后停在 `wait-update-pr`。确认 change-doc、review detail 和目标项目验证后：

```bash
sdr finish --request_id REQ-0001 --message "feat: add feature"
```

`finish` 会：

1. 校验 change-doc gate 有效。
2. 在 request worktree 中 commit。
3. push request 分支。
4. 调用 `tools/pr-create.sh` 创建或复用 PR。
5. 成功后标记 `wait-finish`。

如果 PR connector 失败，会保持或回到 `wait-update-pr`，允许修复 connector 后重试。

`finish` 不会 merge。

## PR 合并确认

PR 合入后运行：

```bash
sdr pr-status --request_id REQ-0001
```

只有 `tools/pr-status.sh` 返回 `merged`，框架才会标记 `finished`。如果返回 `open`，保持 `wait-finish`；如果返回 `missing` 或 `closed`，回到 `wait-update-pr`。

## PR Refresh / Rebase

当 PR 与 base/master 冲突，或需要刷新最新 base：

```bash
sdr pr-refresh --request_id REQ-0001
```

行为：

1. 调用 `tools/pr-status.sh` 观察 PR。
2. fetch base。
3. 尝试 rebase。
4. clean rebase 时更新文档并运行 IntegrationReviewer。
5. 冲突时记录 `pr-conflicts/attempts/NNN-rebase-conflict.md`，派发 RebaseAgent。
6. IntegrationReviewer 通过后回到 `wait-update-pr`。

之后需要再次运行：

```bash
sdr finish --request_id REQ-0001 --message "feat: add feature"
```

rebase 后的非快进推送会使用 `--force-with-lease`。

## Block 与 Resume

block 时先读：

```text
obsidian/changes/<name>/<REQ> recovery.md
obsidian/changes/<name>/status.json
obsidian/changes/<name>/<REQ> agent-journal.md
obsidian/changes/<name>/reviews/*/details/*.json
.sandrone/state/agents/<REQ>.stderr.log
```

修复外部问题后：

```bash
sdr resume --request_id REQ-0001
sdr tick --request_id REQ-0001
```

不要手写 approval JSON，不要修改 reviewer 输出绕过门禁。gate 不可用时必须先修 reviewer/backend/网络/schema。

## Upgrade

先更新本机 CLI 和 skill：

```bash
cd /path/to/Sandrone
scripts/install.sh --force
```

再进入旧 workspace：

```bash
sdr upgrade --dry-run
sdr upgrade
```

普通 `upgrade` 会刷新 `.example.*`、迁移 runtime 文档、更新 registry，但不会覆盖正式 `tools/*.sh`、prompt 或 schema。

如果确认没有自定义 connector/prompt/schema：

```bash
sdr upgrade --default
```

`--default` 会用新版默认实现覆盖正式脚本和 prompt。

## 常见问题

### agent 找不到 Codex CLI

```bash
export SANDRONE_CODEX_APP="/Applications/Codex.app"
sdr doctor
sdr resume --request_id REQ-0001
sdr tick --request_id REQ-0001
```

### git pull 失败

`start` 前会在 `dev/repo` 尝试 `git pull --ff-only`。失败时需要先处理目标仓库同步问题，再：

```bash
sdr resume --request_id REQ-0001
sdr tick --request_id REQ-0001
```

### reviewer gate unavailable

常见原因：

- reviewer backend 网络不可用。
- Codex CLI 路径不可用。
- structured output schema 不匹配。
- reviewer 临时 `CODEX_HOME` 无法读取 auth。

修复后用 `resume` 恢复。不要把 `gate_unavailable` 当作可忽略 warning。
