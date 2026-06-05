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
  checks/
  pr-conflicts/
```

父 request 不再生成父级 `plan.md` 或 `change-doc.md`；slice 不生成单独 `request.md` 或 `pr-doc.md`。slice 的 `<REQ-SNN> plan.md` 同时承载 slice request 与实现计划；父 request 的 `<REQ> pr-doc.md` 是最终 PR/finish 汇总入口。

## 机器状态

| 路径 | 说明 |
| --- | --- |
| `.sandrone/config.toml` | workspace 配置，例如 `parallel_limit`。 |
| `.sandrone/state/requests.tsv` | request 中央索引。 |
| `.sandrone/state/events.ndjson` | 审计事件流。 |
| `.sandrone/state/agents/` | agent stdout、stderr、pid、exit code、hook log。 |
| `.sandrone/state/locks/` | per-request lock，避免 heartbeat 与 hook 重复推进。 |
| `.sandrone/state/sessions.json` | 可见 thread/session registry。 |
| `obsidian/changes/**/status.json` | request/slice 的权威 runtime 状态和 `gates` 记录。 |

`requests.tsv` 用于快速列表，`status.json` 用于具体 request/slice 的权威状态。框架需要保持二者同步；如果旧 workspace 出现列表滞后，通常用 `resume`、`advance` 或 `upgrade` 修复。

## 全局 Registry

Dashboard 读取全局：

```text
~/.sandrone/workspaces.json
```

可以用 `SANDRONE_HOME` 改变目录。`new`、`upgrade`、`list`、`dashboard` 会刷新 registry。进入某个旧 workspace 运行 `sdr list` 或 `sdr upgrade`，就能让它出现在 dashboard 中。
