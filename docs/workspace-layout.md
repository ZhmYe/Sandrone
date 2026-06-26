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
    merge/merge-plan.md
    derived/
    views/
    project.canvas
  agents/
    config/
      implementation-agent.json
      test-reviewer.json
    implementation-agent/
      runs/
    test-reviewer/
      runs/
  tools/
    issue-update.sh
    issue-agent.sh
    check-format.sh
    pr-create.sh
    pr-status.sh
    merge-plan.sh
    pr-merge.sh
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

## Agent Runtime

Sandrone 新 workspace 会按 agent 类型生成独立运行目录:

```text
agents/<agent-kind>/
  runs/
    <timestamp-request-stage-attempt>/
      logs/
        stdout.log
        stderr.log
        hook.log
        events.log
      state/
        pid
        exit
      artifacts/
        runtime.json
        result.json
        review-context/
        merge-queue.tsv
        merge-plan.json
```

agents 共用统一配置目录:

```text
agents/config/
  decomposition-agent.json
  plan-agent.json
  implementation-agent.json
  rebase-agent.json
  decomposition-reviewer.json
  plan-reviewer.json
  test-reviewer.json
  design-reviewer.json
  integration-reviewer.json
  merge-planner.json
```

`agent-kind` 包括 `decomposition-agent`、`plan-agent`、`implementation-agent`、`rebase-agent`、`decomposition-reviewer`、`plan-reviewer`、`test-reviewer`、`design-reviewer`、`integration-reviewer` 和 `merge-planner`。每次运行都会创建一个新的 timestamp run，日志不会互相覆盖。

`agents/config/<kind>.json` 是统一模型/backend/key/base_url 配置源（默认都写入空值）。运行时读取优先级是 shell 环境变量 > `agents/config/*.json` > workspace `.env` 兜底。

Obsidian 只保留人类/AI 需要持续阅读的重要 Markdown、导航、计划、变更说明和当前 merge plan。review context、runtime、stdout/stderr、merge queue、机器 JSON 等中间产物放在 `agents/<kind>/runs/**/artifacts`。`.sandrone` 只保留中央索引、锁、事件流和兼容指针。

## 机器状态

| 路径 | 说明 |
| --- | --- |
| `.sandrone/config.toml` | workspace 配置，例如 `parallel_limit` 和 `auto_merge`。 |
| `.sandrone/state/requests.tsv` | request 中央索引。 |
| `.sandrone/state/events.ndjson` | 审计事件流。 |
| `agents/<kind>/runs/**` | agent/reviewer/merge-planner 的 canonical runtime，包含日志、pid/exit、runtime.json、review context、merge queue 和机器 JSON。 |
| `.sandrone/state/jobs/` | 新 runtime 的兼容指针和旧 workspace fallback。 |
| `.sandrone/state/review-contexts/` | 旧版本兼容路径；新 review context 位于对应 reviewer run 的 `artifacts/review-context/`。 |
| `.sandrone/state/scheduler/merge-queue.tsv` | `agents/merge-planner` 中 merge queue 的兼容副本。 |
| `.sandrone/state/scheduler/merge-plan.json` | `agents/merge-planner` 中最新机器 merge plan 的兼容副本。 |
| `.sandrone/state/scheduler/decisions/*.json` | 每次 `pr-merge` 安全检查和执行结果。 |
| `.sandrone/state/agents/`、`.sandrone/state/reviews/` | 旧版本兼容路径；新 dashboard 和状态收敛优先读取 `state/jobs`，再回退到旧路径。 |
| `.sandrone/state/locks/` | per-request lock，避免 heartbeat 与 hook 重复推进。 |
| `.sandrone/state/sessions.json` | 可见 thread/session registry。 |
| `obsidian/changes/**/status.json` | request/slice 的权威 runtime 阶段状态、阻塞原因、worktree/branch/PR 路径等机器状态。 |
| `obsidian/changes/**/*.md` frontmatter | 阶段文档提交状态、format/check 摘要和 `gate_*` 门禁状态。 |
| `obsidian/merge/merge-plan.md` | 最新全局合并优先级计划；只解释合并顺序，不审计 PR 实现质量。 |

`requests.tsv` 用于快速列表，`status.json` 用于具体 request/slice 的 runtime 状态，阶段 Markdown frontmatter 用于文档提交、format/check 和 gate 状态。框架需要保持这些状态源同步；如果旧 workspace 出现列表滞后或旧 gate 记录残留，通常用 `resume`、`advance` 或 `upgrade` 修复。

## 全局 Registry

Dashboard 读取全局：

```text
~/.sandrone/workspaces.json
```

可以用 `SANDRONE_HOME` 改变目录。`new`、`upgrade`、`list`、`dashboard` 会刷新 registry。进入某个旧 workspace 运行 `sdr list` 或 `sdr upgrade`，就能让它出现在 dashboard 中。
