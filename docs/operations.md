# 运行、交付与恢复

## 自动化运行

推荐用内置 loop 运行自动化：

```bash
cd /path/to/workspace
sandrone loop start --interval-seconds 900
sandrone dashboard
sandrone loop stop
```

`loop start` 会在后台重复执行自动化循环；每轮都会抓取需求、派发或收敛 active cohort 内的 agent/reviewer，并串行处理 cohort 内 PR 交付与安全合并。只有没有 active cohort 时，loop 才会运行 RequestScheduleAgent/Reviewer 选择下一批最多 `parallel_limit` 个父 request。`loop stop` 是软停止，不会中断正在写代码或评审的子 agent；它只阻止下一轮继续开始。`loop stop --force` 只终止 loop worker 本身，不强杀已派发 worker。

active cohort 位于 `.sandrone/state/scheduler/cohort.json`，运行进度位于 `.sandrone/state/scheduler/cohort-progress.json`。cohort 内父 request 全部 `finished` 或 `blocked` 后，会归档到 `last-cohort.json` / `last-cohort-progress.json` 并允许下一轮重新调度。这样同一批并行 request 可以互不影响地推进；先合入的 PR 如果导致同批另一个 PR 冲突，后者会被 PR 状态门禁退回 implementation/code-review，而不是让新批次提前插入。

状态保存会写 `.sandrone/state/loop/wake` 唤醒 loop worker；如果文件事件漏掉，worker 会按 `--interval-seconds` 兜底巡检。

如果要主动暂停某个需求，用 stop 的 request 形态，它会把 request 标记为 blocked:

```bash
sandrone loop stop --request_id REQ-0001 --reason "pause for manual inspection"
```

恢复 blocked request 用 restart；不指定 request 时会恢复所有 blocked request。恢复后继续自动化请运行 `loop start`:

```bash
sandrone loop restart --request_id REQ-0001
sandrone loop restart
sandrone loop start
```

## PR 交付

自动流程通过 code-review 后进入 `wait-update-pr`。下一轮 loop 会执行 PR 交付：

1. 校验 change-doc gate 有效。
2. 在 request worktree 中 commit。
3. push request 分支。
4. 调用 `tools/pr-create.sh` 创建或复用 PR。
5. 成功后标记 `wait-finish`。

如果 PR connector 失败，会保持或回到 `wait-update-pr`，允许修复 connector 后由下一轮 loop 重试。交付成功后进入 `wait-finish`。

## PR 合并确认与自动合并

PR 合入确认由 loop 调用 `tools/pr-status.sh` 完成：返回 `merged` 会标记 `finished`；返回 `open` 保持 `wait-finish`；返回 `missing` 或 `closed` 回到 `wait-update-pr`。

自动合并每轮最多处理一个 active cohort 内的 `wait-finish` request。只有在 `change-doc` gate 已通过、`tools/pr-status.sh` 返回 `safe` 时，loop 才会调用 `tools/pr-merge.sh`。`unsafe` 会把对应 request/slice 退回 implementation/code-review，`open` 或 `unsupported` 只记录判断结果并等待下一轮，不会强行 merge。

## PR 状态退回

当 PR 与 base/master 冲突，或需要刷新最新 base，loop 不在外层直接 rebase。它会用 `tools/pr-status.sh` 判断状态，并在不可安全合并时退回实现阶段：

1. 调用 `tools/pr-status.sh` 观察 PR。
2. 返回 `unsafe` 时，记录 PR 状态门禁结果。
3. 把父 request 或最后一个可修复 slice 退回 `code-review-rejected`。
4. 下一轮 loop 派发 ImplementationAgent，在 worktree 中处理 PR outdated、base/master drift、冲突或平台检查失败。
5. 修复后重新运行 format/check、TestReviewer 和 DesignReviewer。
6. 通过后回到 `wait-update-pr`，由下一轮 loop 更新 PR。

之后下一轮 loop 会重新执行 PR 交付；如果更新已存在 PR 需要非快进推送，交付脚本应使用安全的 `--force-with-lease` 或平台等价能力。

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
sandrone loop restart --request_id REQ-0001
```

`loop restart` 不会伪造 approval。它只把 `blocked` 改回下一步可执行状态；下一次 `loop start` 会继续推进：

- 如果 blocked 来自 reviewer/backend/schema/network 的 `gate_unavailable`，恢复到 `decomposition-submitted`、`plan-submitted` 或 `change-doc-submitted`，下一次 loop 重跑 reviewer。
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
sandrone loop restart --request_id REQ-0001
sandrone loop start
```

### git pull 失败

`start` 前会在 `dev/repo` 尝试 `git pull --ff-only`。失败时需要先处理目标仓库同步问题，再：

```bash
sandrone loop restart --request_id REQ-0001
sandrone loop start
```

### reviewer gate unavailable

常见原因：

- reviewer backend 网络不可用。
- Codex CLI 路径不可用。
- structured output schema 不匹配。
- reviewer 临时 `CODEX_HOME` 无法读取 auth。

修复后用 `resume` 恢复。不要把 `gate_unavailable` 当作可忽略 warning。
