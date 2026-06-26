# 运行、交付与恢复

## 自动化运行

最简单的周期任务是定时执行：

```bash
cd /path/to/workspace
sdr tick
```

`tick` 不会运行 `finish`。默认也不会 merge；只有显式开启 `auto_merge` 后，才会在 `wait-finish` request 中每轮最多选择一个执行安全合并检查。

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

## 可选自动合并

默认流程不会自动 merge。需要机器人合并时，可以单次显式运行：

```bash
sdr pr-merge --request_id REQ-0001 --auto-merge
```

也可以开启 tick 调度：

```toml
# .sandrone/config.toml
auto_merge = true
```

或：

```bash
SANDRONE_AUTO_MERGE=1 sdr tick
sdr tick --auto-merge
```

自动合并调度每轮最多处理一个 `wait-finish` request，但不会把 request 完成事件当成合并触发器。开启后，tick 会先刷新所有候选 PR 的轻量状态，写入全局队列和计划:

- `.sandrone/state/scheduler/merge-queue.tsv`
- `.sandrone/state/scheduler/merge-plan.json`
- `obsidian/merge/merge-plan.md`

其中 `.sandrone/state/scheduler/*` 是兼容副本；canonical merge planner 运行产物位于 `agents/merge-planner/runs/**/artifacts/`。随后调用 `tools/merge-plan.sh` 选择本轮优先合并的一个 request。这个脚本只决定队列优先级，不审计实现质量，不 merge，不 push。`pr-merge` 只有在 `change-doc` gate 已通过、merge-plan 返回 `ready_for_merge`、`tools/pr-status.sh` 返回 `safe` 时才会调用 `tools/pr-merge.sh`。`open`、`unsafe`、`unsupported`、缺少开关或队列未就绪都会只记录计划和 scheduler decision，不会执行 merge。

## PR Refresh / Rebase

当 PR 与 base/master 冲突，或需要刷新最新 base：

```bash
sdr pr-refresh --request_id REQ-0001
```

行为：

1. 调用 `tools/pr-status.sh` 观察 PR。
2. fetch base。
3. 尝试 rebase。
4. clean rebase 时更新文档并派发 IntegrationReviewer worker。
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
.sandrone/state/jobs/<REQ>/agent/current/issue-agent/stderr.log
.sandrone/state/jobs/<REQ>/<stage>/<attempt>/<reviewer>/stderr.log
agents/<kind>/runs/**/logs/stderr.log
```

修复外部问题后：

```bash
sdr resume --request_id REQ-0001
sdr tick --request_id REQ-0001
```

`resume` 不会伪造 approval。它只把 `blocked` 改回下一步可执行状态：

- 如果 blocked 来自 reviewer/backend/schema/network 的 `gate_unavailable`，恢复到 `decomposition-submitted`、`plan-submitted`、`change-doc-submitted` 或 `integration-review-submitted`，下一次 `tick` 重跑 reviewer。
- 如果 blocked 来自 reviewer finding、format/check 失败、实现未完成或超过最大轮次，恢复到对应 review-rejected/planning 状态，下一次 `tick` 派发 agent 修复。

不要手写阶段文档 frontmatter 的 `gate_*` 字段，不要恢复旧版 approval 记录，也不要修改 reviewer 输出绕过门禁。gate 不可用时必须先修 reviewer/backend/网络/schema。

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

普通 `upgrade` 会刷新 `.example.*`、迁移 runtime 文档、更新 registry、补齐阶段 Markdown 的 Sandrone frontmatter，并清理旧 `.sandrone/state/agents/*.success` marker；它不会覆盖正式 `tools/*.sh`、prompt 或 schema。

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
