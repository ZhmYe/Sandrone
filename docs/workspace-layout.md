# Workspace 结构

一个 managed workspace 是目标仓库外面的一层自动开发框架。

```text
<workspace>/
  dev/
    repo/
    worktrees/
  obsidian/
    project.md
    relations.md
    codegraph/context.md
    changes/
    derived/
    views/
    project.canvas
  tools/
    issue-update.sh
    issue-agent.sh
    check-format.sh
    pr-create.sh
    pr-status.sh
    prompts/
    schemas/
  .sandrone/
    config.toml
    state/
  .env
```

## 目标仓库

| 路径 | 说明 |
| --- | --- |
| `dev/repo` | 目标仓库主副本。`new --url` 会 clone 到这里；`new --name` 会创建本地 Git 仓库。 |
| `dev/repo/.codegraph` | CodeGraph 索引目录。 |
| `dev/worktrees/<REQ>` | request 或 slice 的隔离 worktree。agent 只能在这里开发代码。 |

`start` 或自动 implementation 派发前，会在有 remote 的非空 `dev/repo` 尝试 `git pull --ff-only`。失败时会 block，避免基线落后或分叉导致后续工作不可信。

## Obsidian 文档

每个 request 的文档位于：

```text
obsidian/changes/<YYYY-MM-DD-request-name>/
  REQ-0001 index.md
  REQ-0001 request.md
  REQ-0001 decomposition.md
  REQ-0001 pr-doc.md
  REQ-0001 agent-journal.md
  decomposition.json
  dag.json
  status.json
  slices/
    S01/
      REQ-0001-S01 index.md
      REQ-0001-S01 plan.md
      REQ-0001-S01 change-doc.md
      REQ-0001-S01 agent-journal.md
      slice.json
      status.json
      reviews/
  reviews/
  pr-conflicts/
```

父 request 不再生成父级 `plan.md` 或 `change-doc.md`；slice 不生成单独 `request.md` 或 `pr-doc.md`。slice 的 `<REQ-SNN> plan.md` 同时承载 slice request 与实现计划；父 request 的 `<REQ> pr-doc.md` 是最终 PR/finish 汇总入口。

## 机器状态

| 路径 | 说明 |
| --- | --- |
| `.sandrone/config.toml` | workspace 配置，例如 `parallel_limit`。 |
| `.sandrone/state/requests.tsv` | request 中央索引。 |
| `.sandrone/state/events.ndjson` | 审计事件流。 |
| `.sandrone/state/jobs/` | agent/reviewer 的统一运行时目录，包含 `pid`、`exit`、`stdout.log`、`stderr.log`、`hook.log`、`events.log` 和 `runtime.json`。阶段完成状态写在对应 Markdown 文档的 Sandrone frontmatter。 |
| `.sandrone/state/review-contexts/` | 每轮 reviewer 的轻量索引目录，包含 `artifact-index.md`、`changed-files.txt`、`diff-stat.txt` 和 `test-summary.txt`；长文档只在 index 中以原始路径引用，不再复制。 |
| `.sandrone/state/agents/`、`.sandrone/state/reviews/` | 旧版本兼容路径；新 dashboard 和状态收敛优先读取 `state/jobs`，再回退到旧路径。 |
| `.sandrone/state/locks/` | per-request lock，避免 heartbeat 与 hook 重复推进。 |
| `.sandrone/state/sessions.json` | 可见 thread/session registry。 |
| `obsidian/changes/**/status.json` | request/slice 的权威 runtime 阶段状态、阻塞原因、worktree/branch/PR 路径等机器状态。 |
| `obsidian/changes/**/*.md` frontmatter | 阶段文档提交状态、format/check 摘要和 `gate_*` 门禁状态。 |

`requests.tsv` 用于快速列表，`status.json` 用于具体 request/slice 的 runtime 状态，阶段 Markdown frontmatter 用于文档提交、format/check 和 gate 状态。框架需要保持这些状态源同步；如果旧 workspace 出现列表滞后或旧 gate 记录残留，通常用 `resume`、`advance` 或 `upgrade` 修复。

## 全局 Registry

Dashboard 读取全局：

```text
~/.sandrone/workspaces.json
```

可以用 `SANDRONE_HOME` 改变目录。`new`、`upgrade`、`list`、`dashboard` 会刷新 registry。进入某个旧 workspace 运行 `sdr list` 或 `sdr upgrade`，就能让它出现在 dashboard 中。
