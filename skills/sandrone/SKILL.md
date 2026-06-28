---
name: sandrone
description: Use when the user asks Codex to create, clone, run, restart, stop, dashboard, upgrade, recover, review, or manage software work with sandrone workspaces, especially when explicit approval gates, request IDs, Chinese change templates, isolated worktrees, loop automation, global workspace registry, recovery docs, target project checks, no-commit/no-push agent boundaries, or PR delivery matter.
metadata:
  short-description: Run sandrone approval-gated workspaces
---

# Sandrone

Sandrone 是一个 approval-gated 自动开发外框架。CLI 负责 workspace、request/slice 状态机、Obsidian 文档包、CodeGraph context、review gate、worktree、PR 交付和 dashboard；Codex agent 负责拆解、计划、实现、验证和写文档。

## 必做第一步: 安装或验证 CLI

Before any workspace command, verify that the CLI is installed:

```bash
sandrone --help
```

如果命令不存在，先停止并告诉用户需要安装 Rust CLI。只有在用户明确批准后才安装:

```bash
curl -fsSL https://raw.githubusercontent.com/ZhmYe/Sandrone/master/scripts/bootstrap.sh | sh
```

如果当前已经 clone 了本仓库:

```bash
scripts/install.sh --force
```

Do not run workspace commands until `sandrone --help` succeeds. 安装脚本会安装 `sandrone` skill、`obsidian-change-trace` skill，并尽力安装/配置 CodeGraph。CodeGraph 失败时提示用户参考 `docs/codegraph.md`。

## 先判断当前角色

主会话或 heartbeat:

- 使用 `sandrone loop start`、`sandrone loop restart`、`sandrone loop stop` 和 `sandrone dashboard` 作为普通用户入口。
- `tick`、`advance`、`resume`、`block`、`finish`、`pr-*`、review 命令是内部/advanced 命令，主要给 hook、connector、测试和恢复使用；不要作为默认交互路径推荐给用户。
- 不手写 `.sandrone` 状态、review JSON、阶段文档 frontmatter 的 `gate_*` 字段或 PR 状态。
- 自动流程由 loop 统一处理 PR 交付；默认不会自动 merge。
- review gate 和 agent 一样是异步状态机：外层 loop 只派发 reviewer worker；后台结束后由 hook 或下一轮 loop 收敛。

子 agent:

- 如果环境里有 `SANDRONE_AGENT_PHASE`，优先遵守启动 prompt、`tools/prompts/issue-agent.md` 和当前 phase prompt。
- 不要在子 agent 中调用 `submit`、`approve`、`reject`、`plan-review`、`code-review`、`start`、`finish`、commit、push 或 PR。
- 不要把本 skill 当作完整运行手册反复阅读；本文件只是短入口，phase 细则在 prompt 和 runtime 文件里。

## 上下文预算

必要 skill/plugin 可以用，但必须按需、分层、分轮读取。不要一轮读完完整 skill、完整 docs、全部 review 历史、全部 slice 文档、完整事件流或全仓源码。

推荐读取顺序:

1. 当前命令输出、`status.json`、`recovery.md`、当前 request/slice index。
2. 当前 phase 主产物: request/decomposition/dag/plan/change-doc。
3. `obsidian/codegraph/context.md` 和 CodeGraph 定位到的少量源码。
4. `agent-journal.md` 最近几轮记录，不默认全文读取。
5. 启动 prompt 列出的 latest review summary/detail；如果最新 attempt 是 `gate_unavailable=true`，再读 latest actionable non-unavailable detail。运行日志和中间产物优先读 `agents/<kind>/runs/**`，`.sandrone/state/jobs` 只作为兼容兜底。
6. 仅当上述信息不足，再打开对应 docs 或 skill 的相关小节。

主会话保留用户 Codex skill/plugin。自动化子 agent 默认不继承用户个人 Codex config、skill 和插件，以免自动读入大量无关上下文；Sandrone 会把 phase prompt、CodeGraph/Obsidian 路径和脚本能力显式传给子 agent。只有确实需要子 agent 继承个人插件/skill 时，才在 workspace `.env` 设置:

```bash
SANDRONE_AGENT_IGNORE_USER_CONFIG=0
```

## 常用命令

```bash
sandrone loop start
sandrone loop restart --request_id REQ-0001
sandrone loop stop --request_id REQ-0001 --reason "manual pause"
sandrone loop stop
sandrone dashboard
```

`sdr` 是 `sandrone` 的短别名，但文档和自动化提示词默认使用完整命令名。

## 自动流程摘要

```text
update
-> RequestScheduleAgent + RequestScheduleReviewer
-> decomposition agent
-> DecompositionReviewer
-> materialize slice DAG
-> plan agent
-> PlanReviewer
-> implementation agent
-> format/check gate
-> TestReviewer + DesignReviewer
-> wait-update-pr
-> PR delivery(commit/push/PR)
-> wait-finish
-> pr-status(merged => finished)
```

推荐主控是 `sandrone loop start --interval-seconds 900`。每轮先抓取需求；只有没有 active cohort 时，才由 RequestScheduleAgent 选择最多 `parallel_limit` 个候选父 request，并由 RequestScheduleReviewer 审核本批调度。通过后写入 `.sandrone/state/scheduler/cohort.json`。active cohort 存在期间，loop 只推进 cohort 内 request/slice/PR；cohort 内父 request 全部 `finished` 或 `blocked` 后，下一轮才会调度新 cohort。它只审并行选择和依赖边界，不审代码质量。

`sandrone loop stop` 是软停止，不强杀正在运行的 agent/reviewer；`sandrone loop stop --request_id <REQ>` 会主动把该 request 标记 blocked。`sandrone loop restart` 只负责 resume blocked request，继续自动化仍由 `sandrone loop start` 负责。

PR 交付和合并默认自动串行执行。每轮 loop 最多推进一个 active cohort 内 PR 交付/合并动作：先处理 `wait-update-pr` 的 commit/push/PR 创建或更新；没有待交付 PR 时，处理一个 `wait-finish` 的 PR 状态检查和合并。实现质量、是否可以提交 PR 和是否满足需求完整性由最后一个 slice 的 code-review gate 负责。真正合并前仍必须由 `tools/pr-status.sh` 返回 `safe`；`open`、`unknown`、`unsupported` 不合并，`unsafe` 会把对应 request/slice 退回 implementation，让 ImplementationAgent 处理 PR 过期、冲突或 base/master drift，然后重新通过 format/check 与 code-review。

PR 过期或冲突走退回闭环:

```text
wait-finish -> pr-status unsafe -> implementation agent -> code-review -> wait-update-pr
```

默认 `.sandrone/config.toml` 中 `parallel_limit = 1`。需要并发时用 `sandrone loop start --parallel-limit 2` 或改配置；request-schedule agent 会在最多 N 个候选中选择尽量独立、依赖清楚、冲突概率可接受的 request，不必选满 N 个。PR 合并始终由 loop 串行执行，并被限制在 active cohort 内。active cohort 的运行进度会写入 `.sandrone/state/scheduler/cohort-progress.json`；状态保存会写 loop wake 文件，worker 被文件事件唤醒继续推进，如果事件漏掉才按 `--interval-seconds` 兜底巡检。

Agent/reviewer 的模型和 backend 推荐在 `agents/config/<kind>.json` 配置。每个文件包含 `agent_backend`、`model`、`reasoning_effort`、`api_key` 和 `base_url`；读取优先级是 shell 环境变量 > `agents/config/*.json` > workspace `.env` 兜底。Reviewer 也是独立 agent，使用同一套配置结构。

## 关键边界

- 审批/门禁是显式状态化流程，记录在对应阶段 Markdown 文档 frontmatter 的 `gate_*` 字段；不得手写这些字段、修改 reviewer 输出，或恢复旧 `status.json.gates` / `approvals/` 来伪造门禁。
- Reviewer gate 必须由外层 loop 派发和收敛，不得在子 Codex 中嵌套调用 reviewer。运行态会写成 `decomposition-review-running`、`plan-review-running` 或 `code-review-running`；旧 workspace 的 `integration-review-running` 仅作兼容状态。canonical 运行日志和中间产物在 `agents/<reviewer>/runs/<timestamp-REQ-stage-attempt>/`，`.sandrone/state/jobs` 和 `.sandrone/state/reviews` 仅作兼容兜底。
- Reviewer 输出必须是结构化 JSON。`SANDRONE_REVIEW_CONTEXT` 是轻量隔离上下文目录；reviewer 必须先读其中的 `artifact-index.md`，再按索引里的原始路径和自动摘要按需读取。TestReviewer 与 DesignReviewer 不得读取其他 reviewer 输出、历史 summary/detail 或 agent journal。对 slice 来说没有独立 `request.md`，plan 就是 slice 的权威 request+plan。
- 默认 `codex-cli` 和 `codex-api` connector 会尽量使用 `SANDRONE_CODEX_MODEL_CATALOG_JSON`、`$CODEX_HOME/models_cache.json` 或 `$HOME/.codex/models_cache.json`，避免 agent/reviewer 在模型列表刷新阶段因为网络超时或 provider `/models` 格式不兼容而失败。若 backend、模型或结构化输出不可用，必须 block，不能绕过 reviewer gate。
- 子 agent 只有在当前 phase 产物、journal、自检和必要验证全部完成后，才能把 `$SANDRONE_AGENT_STATUS_DOC` 的 frontmatter 标记为 `agent_status: submitted`、`agent_ready_for_review: true`。这个状态头只是“可提交外层 review gate”的完成信号，不是 approval，不能替代 reviewer；非零退出且没有有效文档提交状态时必须 block。
- Review 打回后的同 phase retry 可能复用上一轮 Codex session。复用只用于节省上下文和延续判断；本轮 prompt、最新 reviewer finding、当前文件和状态始终优先，仍必须重新通过外层 reviewer gate。
- implementation agent 只能改 `SANDRONE_WORKTREE`，不直接改 `dev/repo`。
- implementation agent 必须运行格式/编译/测试门禁，更新目标项目文档和 change-doc。交付文档中的 checklist 必须全部打勾；无法由当前流程完成的事项不得保留为未勾选 checklist，应移到后续流程/人工事项/阻塞项。
- 如果测试暴露不是由本分支改动导致的已有失败，implementation agent 也要在当前 worktree 中修复并记录 Baseline failure，除非修复不安全。
- PR 状态门禁退回时，ImplementationAgent 必须保留 base/master 新代码和已通过实现语义；不得借冲突修复扩大需求，也不得 commit、push、finish 或 merge。

## 文档索引

这些文档位于 Sandrone 源码仓库 checkout 中；安装态 `~/.codex/skills/sandrone` 默认只包含本 `SKILL.md`。如果当前不在 Sandrone 源码仓库，不要尝试读取这些 `docs/` 文件，改用 workspace 自带的 `tools/prompts/*.md`、`status.json`、`recovery.md` 和 Obsidian 文档包。

在 Sandrone 源码仓库内维护框架时，只读需要的文档:

- `docs/installation.md`: 安装、Codex CLI 路径、`.env`、模型/backend、代理。
- `docs/workflow.md`: request/slice 生命周期、review gate、并发调度、PR 状态退回。
- `docs/commands.md`: CLI 命令参考。
- `docs/workspace-layout.md`: workspace、`.sandrone`、Obsidian 文档包、registry。
- `docs/connectors.md`: `tools/*.sh` 输入输出契约、`codex-cli`/`codex-api`/`claude-code` backend。
- `docs/obsidian.md`: Obsidian vault、derived JSON、Canvas/Base、图谱边界。
- `docs/codegraph.md`: CodeGraph 安装、初始化、context 使用和排障。
- `docs/dashboard.md`: dashboard 展示规则；主列表只展示父 request，详情包含 `需求分析`、`Slice 1`、`Review 结果` 和 PR tab，Markdown 用 `marked`，JSON/review 用 `jsoneditor`。
- `docs/operations.md`: loop 运行、PR 交付、PR 状态、退回修复、upgrade、block/restart。
- `docs/development.md`: 框架自身维护、源码模块、测试命令。

## 默认 Prompt 入口

子 agent 和 reviewer 的细则在 managed workspace 里；这些文件由 `sandrone new/upgrade` 生成，不属于安装态 skill:

- `tools/prompts/issue-agent.md`: 共享 agent 契约、上下文预算、latest review 读取策略。
- `tools/prompts/decomposition-agent.md`: slice DAG 拆解和 DecompositionReviewer 提交前自检。
- `tools/prompts/plan-agent.md`: plan 内容、PlanReviewer 提交前自检。
- `tools/prompts/implementation-agent.md`: 实现、测试、change-doc、PR 状态退回修复、Code Review 提交前自检。
- `tools/prompts/*-reviewer.md`: reviewer 严格输出格式和评审边界。

升级旧 workspace 时普通 `sdr upgrade` 只刷新 `.example.*` 并迁移 runtime，不覆盖正式 connector/prompt/schema；`sdr upgrade --default` 才会用新版默认实现覆盖正式脚本和 prompt。
