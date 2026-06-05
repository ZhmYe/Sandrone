---
name: sandrone
description: Use when the user asks Codex to create, clone, update, tick, plan, implement, review, block, resume, finish, upgrade, approve, dashboard, or manage software work with sandrone workspaces, especially when explicit approval gates, request IDs, Chinese change templates, isolated worktrees, issue-agent automation, global workspace registry, recovery docs, target project checks, no-commit/no-push agent boundaries, or finish-time PR delivery matter.
metadata:
  short-description: Run sandrone approval-gated workspaces
---

# Sandrone

当用户要求 Codex 新建仓库、clone 仓库、同步需求、自动处理 issue、写计划、实现需求、审批、阻塞恢复、查看 dashboard、升级旧 workspace 或完成变更时，使用这个 skill。

## 必做第一步: 安装或验证 CLI

Before any workspace command, verify that the CLI is installed:

```bash
sandrone --help
```

如果命令不存在，先停止并告诉用户这个 skill 还需要 Rust CLI。只有在用户明确批准后，才可以安装 CLI 和本地 skill:

```bash
curl -fsSL https://raw.githubusercontent.com/ZhmYe/Sandrone/master/scripts/bootstrap.sh | sh
```

如果当前已经 clone 了本仓库，可以用:

```bash
scripts/install.sh --force
```

安装脚本会同时安装 `sandrone` skill、随框架打包的 `obsidian-change-trace` skill，并尽力安装/配置 CodeGraph (`@colbymchenry/codegraph`)。它不会安装或覆盖用户本地个人用的 `obsidian-note` skill。如果 CodeGraph 自动安装失败，必须告诉用户手动运行:

```bash
npm install -g @colbymchenry/codegraph
codegraph install -t codex -l global -y
```

或设置:

```bash
export SANDRONE_CODEGRAPH_BIN=/absolute/path/to/codegraph
```

Do not run workspace commands until `sandrone --help` succeeds. 对非空目标仓库，`codegraph --version` 和 `codegraph init -i dev/repo` 也应可用；不可用时先运行 `sandrone doctor` 获取恢复步骤。

## 核心边界

`sandrone` 只做机械动作: 创建 workspace、clone 目标仓库、request 记录、全局 workspace registry、Obsidian change 文档包、CodeGraph context、`status.json.gates` 门禁状态、review 结果、session registry、worktree、blocked/recovery、dashboard 数据和 finish-time commit/push/PR。

Codex CLI 子运行负责思考和交付: 填写 `$SANDRONE_DECOMPOSITION`/`dag.json`/小型需求覆盖说明、填写 `$SANDRONE_PLAN`、实现代码、运行目标项目检查、写简洁 `$SANDRONE_CHANGE_DOC`、维护 `obsidian/changes/<change-name>/<REQ> index.md` 导航、根据 reviewer 结果修复、记录 `$SANDRONE_AGENT_JOURNAL`。

自动化无人值守流程必须通过 reviewer gate 推进，不得直接跳过审批。默认 connector 都可替换:

- `tools/issue-update.sh`
- `tools/issue-agent.sh`
- `tools/check-format.sh`
- `tools/decomposition-review.sh`
- `tools/plan-review.sh`
- `tools/test-review.sh`
- `tools/design-review.sh`
- `tools/integration-review.sh`
- `tools/rebase-agent.sh`
- `tools/pr-create.sh`
- `tools/pr-status.sh`

## 自动 Heartbeat 流程

主 session 或 heartbeat 应调用:

```bash
sandrone tick
```

`tick` 做短主控和兜底恢复:

1. 运行 `update`。
2. 刷新已结束 agent 的状态。
3. 找出 eligible request；如果传了 `--request_id`，只处理该 request。
4. 必要时为父 request 创建 decomposition 文档包；每个 request 都先形成 slice DAG，小需求通常只有 `S01`。
5. DecompositionReviewer 通过后 materialize slice request，并在并发上限内派发依赖已满足的 slice。
6. 对漏掉 hook 的已结束 agent，执行同 `advance` 一样的 gate 推进。
7. slice code-review 通过后标记 `slice-finished`；全部 slice 完成后父 request 标记 `wait-update-pr`；tick 永远不运行 `finish`。

新 workspace 的 `.sandrone/config.toml` 默认包含 `parallel_limit = 1`，也就是同一时间最多自动处理 1 个 issue 或 slice。`tick` 会统计当前 `decomposition-agent-running`、`planning-agent-running`、`implementation-agent-running`、`rebase-agent-running` 和 legacy `agent-running` request，只有剩余槽位才会派发新的 request/slice。需要并行处理多个 issue 或 slice 时，可以修改配置，或单次运行:

```bash
sandrone tick --parallel-limit 2
```

运行中的 request/slice 保持 `decomposition-agent-running`、`planning-agent-running`、`implementation-agent-running` 或 `rebase-agent-running`，不会重复派发。agent stdout、stderr、pid、exit code 和 hook log 写入 `.sandrone/state/agents/`。agent wrapper 写入 exit code 后会立即调用 `sandrone advance --request_id <REQ>`；父 request decomposition 通过后，advance 会立刻派发第一个可运行 slice，因此正常情况下不需要等下一次 heartbeat 才 review 或继续推进。需要定时时，让 Codex heartbeat、cron 或其他调度器每 15 分钟调用一次 `sandrone tick` 发现新需求和兜底恢复。

`advance` 是单 request 推进器:

```bash
sandrone advance --request_id <REQ-0001>
```

它不运行 issue update，不扫描全部 request，只在 per-request lock 下刷新一个 request、提交 gate、执行 reviewer、创建 worktree、派发下一 phase 或标记 `slice-finished`/`wait-update-pr`/`blocked`。父 request 顺序是 `decomposition-agent -> DecompositionReviewer -> materialize slices`；每个 slice 顺序是 `planning-agent -> PlanReviewer -> implementation-agent -> code-review`。hook 和 heartbeat 同时触发时，拿不到 `.sandrone/state/locks/<request_id>.lock/` 的一方会跳过。

运行环境或 reviewer 可用性不确定时，先运行:

```bash
sandrone doctor
```

`doctor` 检查 workspace、Git、Codex CLI、GitHub CLI、CodeGraph CLI、target repo、agent/reviewer connector、review schema、CodeGraph index 和事件流目录。它显示 warning/fail，不得 panic。

所有关键状态变化都会追加到 `.sandrone/state/events.ndjson`。该文件是审计、前端展示和恢复分析的稳定事件流；不要让 agent 手动改写它。

全局 workspace registry 默认写入 `~/.sandrone/workspaces.json`，可用 `SANDRONE_HOME` 覆盖目录。`new`、`upgrade`、`list` 和 `dashboard` 会刷新 registry。`sandrone list` 在当前 workspace 内只列出当前项目的 request；`sandrone dashboard` 会读取 registry，展示本机所有已登记 workspace。

每个 managed workspace 同时是一个独立 Obsidian vault:

- `.obsidian/`: Obsidian 配置目录，不放笔记正文。
- `obsidian/project.md`: 当前 workspace 的 Obsidian 根节点，按日期索引父 request，并且只直接链接父 `<REQ> index.md`。
- `obsidian/relations.md`: 轻量关系入口，当前只记录人工/agent 可读关系，不参与调度算法。
- `obsidian/derived/requests.json`: 从 request/status 派生的轻量 request 索引，适合 agent 先读，避免扫描全部历史文档。
- `obsidian/derived/slices.json`: 从 decomposition/DAG 和 materialized slice 派生的轻量 slice 索引，适合 agent 快速判断 slice 依赖和状态。
- `obsidian/views/*.base`: Obsidian Bases 视图定义，从笔记 properties 聚合，是派生视图，不承载权威状态。
- `obsidian/project.canvas`: 从 project、request 和 slice DAG 派生的 JSON Canvas，主要用于人类观察；Canvas 边也应保持 `project -> parent request -> slice`；AI 应优先读取 `derived/*.json`、`dag.json` 和 `decomposition.json`。
- `obsidian/changes/<change-name>/`: 父 request 的需求分析包，包含 `<REQ> index.md`、`<REQ> request.md`、`<REQ> decomposition.md`、`decomposition.json`、`dag.json`、`<REQ> pr-doc.md`、`<REQ> agent-journal.md`、slice 子目录、review detail、`status.json.gates` 和 `<REQ> recovery.md`。正常拆解流程下父 request 不生成父级 `<REQ> plan.md` 或 `<REQ> change-doc.md`；旧的直接 `plan` 兼容路径若已存在这些文件，可以继续读。
- `obsidian/changes/<change-name>/slices/<SNN>/`: slice 的执行包，包含 `<REQ-SNN> index.md`、`<REQ-SNN> plan.md`、`<REQ-SNN> change-doc.md`、`<REQ-SNN> agent-journal.md`、review detail、`status.json.gates`、checks 和 `slice.json`。slice 不生成单独 `<REQ-SNN> request.md`，因为 `$SANDRONE_PLAN` 同时承载 slice request 与计划；slice 也不生成 `<REQ-SNN> pr-doc.md`，最终 PR 文档属于父 request。
- `obsidian/changes/<change-name>/<REQ> index.md`: request/slice 的导航笔记，记录关系、状态摘要、下一步、到 `<REQ> agent-journal.md` 和各阶段总文档的链接；阶段文档文件名必须带 request id，便于 Obsidian 标签页、文件树和图谱区分。
- `obsidian/codegraph/context.md`: 框架用 CodeGraph 生成的默认代码理解入口。
- `.sandrone/`: 机器索引、事件流、锁、agent pid/日志、session registry 和全局 workspace registry。

不要把完整 plan、完整 change-doc 或完整 reviewer JSON 复制进 `<REQ> index.md`；这些文件已经在同一个 Obsidian change 目录中，用带 request id 的文件名和短摘要连接即可。agent 必须优先使用 `$SANDRONE_REQUEST`、`$SANDRONE_DECOMPOSITION`、`$SANDRONE_PLAN`、`$SANDRONE_CHANGE_DOC`、`$SANDRONE_AGENT_JOURNAL`，不要手拼 `plan.md` 等旧短文件名。

Obsidian 图谱主链路必须保持清晰: `project.md -> 父 request index -> slice index -> 阶段总文档`。`project.md` 不得直接 wikilink slice index、阶段文档、Base view、Canvas、derived JSON 或 CodeGraph context；这些辅助文件只在 project note 中以普通路径说明。只有父 request index 可以把上级导航 wikilink 到 `project.md`；slice index 的上级导航指向父 request index，阶段文档只指向当前 request/slice index 或阶段相关文档，不再反向链接 project。

`sandrone obsidian-refresh` 可以随时重新同步 request/slice 导航笔记，并派生 `obsidian/project.md`、`relations.md`、`derived/*.json`、`views/*.base` 和 `project.canvas`。这些派生文件由框架生成；不要让 agent 手写 Canvas/Base 来表达业务状态。

不要在 `sandrone` 源码仓库根目录把本框架初始化成 managed workspace。CLI 会拒绝这种 `new` 操作；测试真实目标项目时，必须切换到单独的外层目录。

Dashboard 主列表只展示父 request，不把 materialized slice 当作独立需求刷在列表里。父 request 详情下面有内部 tab: `需求分析`、`Slice 1`、`Slice 2`、`PR` 等。`需求分析` 与各个 slice tab 同级，展示父 `<REQ> request.md`、`<REQ> decomposition.md` 和 Decomposition Review；slice tab 展示该 slice 的 `Plan -> Implementation` 流水线，Plan Review 和 Code Review 折叠到对应阶段下方的 `Review 结果` tab；`PR` tab 展示父 `<REQ> pr-doc.md`、PR refresh 冲突记录和 Integration Review，只有出现 PR refresh/rebase/integration 记录时才显示 `PR Refresh` 与 `Integration Review` 节点。review tab 必须读取不可变 detail JSON: `reviews/decomposition-review/details/*.json`、`reviews/plan-review/details/*.json`、`reviews/code-review/details/*.json` 和 `reviews/integration-review/details/*.json`，按 `001-*`、`002-*` 等 attempt 分组展示每轮 reviewer 结果。不要依赖会被覆盖的 `summary.json` 来展示 review 细节；`<REQ> recovery.md` 不进入主 stage 区域。

Dashboard request 列表必须优先展示未完成项；`finished` request 稳定排在列表后面，同一组内保持原始 request 顺序。

Dashboard 的 request 区域应是纵向列表。Markdown 文件用 `marked`、`DOMPurify` 和 `highlight.js` 呈现；JSON 文件和 reviewer detail 用 `jsoneditor` 只读 view 呈现。CDN 不可用时必须回退到纯文本，不影响监控。

短命令 `sdr` 是 `sandrone` 的别名，例如 `sdr dashboard`、`sdr list`。

源码维护边界:

- `src/main.rs`: CLI 命令分发、workspace 初始化、tick/agent 编排和少量流程胶水。
- `src/state.rs`: `requests.tsv`、`sessions.json`、`status.json`、approval 和事件流读写。
- `src/codegraph.rs`: CodeGraph CLI 检查、`dev/repo/.codegraph` 初始化和 `obsidian/codegraph/context.md` 生成。
- `src/obsidian.rs`: workspace 独立 Obsidian vault 目录和 request 导航笔记同步。
- `src/review_gate.rs`: DecompositionReviewer、PlanReviewer、TestReviewer、DesignReviewer、IntegrationReviewer 的门禁执行、JSON 规范化和 review 结果写入。
- `src/delivery.rs`: `finish` 阶段的 git commit/push、PR body 渲染和 PR connector 调用。
- `src/doctor.rs`: 环境诊断命令。
- `src/registry.rs`: 全局 `workspaces.json` 读写、刷新和当前 workspace 登记。
- `src/dashboard.rs`: dashboard HTTP 服务、JSON 数据模型和 stage/review artifact 映射。
- `src/defaults.rs`: workspace 默认目录、默认 connector、prompt、schema 和 runtime Markdown 的生成/升级。
- `src/utils.rs`: 时间、路径、JSON 文本解析、Markdown/TSV 转义等共享小工具。
- `src/assets.rs`: 编译期引用模板和静态资产。
- `assets/dashboard/index.html`: dashboard 前端页面，属于固定静态资产，不是 workspace 模板。
- `templates/prompts/*.md`: 默认 agent/reviewer prompt。
- `templates/scripts/*.sh`: 默认 connector 脚本模板。
- `templates/runtime/*.md`: request、plan、change-doc、agent-journal 初始模板。
- `templates/schemas/*.json`: 默认结构化输出 schema。

对非空目标仓库，`new --url` 和计划前检查会自动尝试运行 `codegraph init -i dev/repo`，让 CodeGraph MCP 能读取目标仓库索引，并用 `codegraph context -p dev/repo ...` 刷新 `obsidian/codegraph/context.md`。如果 CodeGraph CLI 不存在或初始化失败，流程必须给出明确恢复命令，不得 panic；agent/reviewer 需要在 preflight、journal 或 finding 中记录风险。

## Connector Contract

所有可替换脚本都必须遵守稳定输入输出契约，保证 issue-agent prompt 可以保持通用，不依赖 GitHub/Jira/内部系统的特定字段。

如果需要按阶段调整模型，工作区根目录 `.env`（或 `SANDRONE_ENV_FILE` 指向的文件）是默认配置源。脚本会按以下顺序解析：
`SANDRONE_*` 显式环境变量 -> `.env` -> `CODEX_HOME/config.toml`（兜底模型与 reasoning effort）。

默认 reviewer connector 会为每次评审创建临时 `CODEX_HOME`。它只复制用户的 `auth.json`，并写入一个禁用插件和 hooks 的最小 `config.toml`；模型、reasoning effort 仍按上面的优先级解析后通过命令行传入。不要把用户完整 `config.toml` 复制进临时 reviewer home，否则启用但未复制缓存的 Browser、GitHub、oh-my-codex 等插件会触发远程同步，容易因为网络、GitHub rate limit 或插件缓存缺失造成 `gate_unavailable=true`。只有在用户显式设置 `SANDRONE_REVIEW_CODEX_HOME` 时，才使用用户提供的完整 Codex home。

常用字段：

- `SANDRONE_DECOMPOSITION_AGENT_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_PLAN_AGENT_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_IMPLEMENTATION_AGENT_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_REBASE_AGENT_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_PLAN_REVIEWER_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_TEST_REVIEWER_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_DESIGN_REVIEWER_MODEL` / `_REASONING_EFFORT`
- `SANDRONE_INTEGRATION_REVIEWER_MODEL` / `_REASONING_EFFORT`

- `tools/issue-update.sh`: stdout 输出零行或多行 TSV，无 header。字段必须是 `external_id<TAB>source<TAB>title<TAB>body<TAB>url`。`external_id` 必须稳定，`source` 是短平台名，`title` 是规范化需求名称，`body` 是完整需求描述，`url` 可为空。
- `tools/issue-agent.sh`: 输入来自 `SANDRONE_*` 环境变量和 runtime 文档。它是 agent 后端 connector，不是业务提示词本身；默认实现会组合 `tools/prompts/issue-agent.md` 共享 agent 契约，以及 `tools/prompts/decomposition-agent.md`、`tools/prompts/plan-agent.md` 或 `tools/prompts/implementation-agent.md` 的 phase-specific prompt。`SANDRONE_AGENT_PHASE=decomposition` 时只写 `$SANDRONE_DECOMPOSITION`、`decomposition.json`、`dag.json`、Obsidian 导航和 `$SANDRONE_AGENT_JOURNAL`；`planning` 时只写 `$SANDRONE_PLAN`；`implementation` 时只在 `SANDRONE_WORKTREE` 写代码并更新简洁 `$SANDRONE_CHANGE_DOC`。实际 Obsidian 阶段文档文件名带 request/slice id，例如 `REQ-0001 request.md`、`REQ-0001 decomposition.md`、`REQ-0001-S01 plan.md`、`REQ-0001-S01 change-doc.md`；agent 不得手拼旧短文件名。成功/失败由退出码表示，失败时 stderr 必须给出可恢复原因。默认 agent/reviewer connector 不写死 Codex.app 路径；需要从普通终端运行时，可以把 `codex` 放进 `PATH`，或设置 `SANDRONE_CODEX_BIN` 指向可执行文件，或设置 `SANDRONE_CODEX_APP` 指向 Codex app bundle。
- `tools/check-format.sh`: code-review 前置检查 connector，不是 reviewer。支持 `--format` 和 `--check`。默认 Rust 实现会在 `SANDRONE_WORKTREE` 有 `Cargo.toml` 时运行 `cargo fmt`、`cargo fmt --check`、`cargo check` 和 `cargo clippy --all-targets --all-features -- -D warnings`；非 Rust 项目默认明确 skip。`code-review` 会先运行 `tools/check-format.sh --check`，失败时不会调用 TestReviewer/DesignReviewer，而是写入 `checks/format-check.md`，把 request/slice 标记为 `code-review-rejected` 并回到 implementation。
- `tools/decomposition-review.sh`、`tools/plan-review.sh`、`tools/test-review.sh`、`tools/design-review.sh`、`tools/integration-review.sh`: stdout 必须是一个符合 `tools/schemas/review-result.schema.json` 的 JSON 对象。非法 JSON、空输出或脚本失败都会变成 `gate_unavailable=true` 的 blocking review；自定义 reviewer 如果无法可靠评审，也必须返回 `gate_unavailable=true` 或非 0 退出。每个 reviewer 必须返回 `recommended_next_phase`，只能是 `planning`、`implementation` 或 `blocked`。
- reviewer 输入必须来自隔离的 `$SANDRONE_REVIEW_CONTEXT`，其中只包含 request、decomposition、DAG、plan、change-doc、CodeGraph context、Obsidian note、status 和 `status.json.gates`，不包含 `reviews/` 或 agent journal。code-review 中 TestReviewer 与 DesignReviewer 必须独立重新评审，不得读取其他 reviewer 输出、历史 summary/detail、上一轮 review 意见或 `$SANDRONE_REVIEW_FORBIDDEN_PATHS`；DesignReviewer 不得依赖 TestReviewer 结论。
- `tools/pr-create.sh`: 必须先判断当前平台/仓库是否支持创建 PR，再检查 base/head 是否已经存在 PR。成功时 stdout 输出一个 TSV 行: `created<TAB>url` 或 `existing<TAB>url`；旧脚本只输出 URL 仍按 created 兼容。失败时 stderr 输出原因。它不得 merge。
- `tools/pr-status.sh`: 由 `pr-status` 和 `pr-refresh` 调用，只观察 PR 状态，不修改代码、分支或 PR。成功时 stdout 输出 `status<TAB>url<TAB>detail`；status 推荐 `open`、`missing`、`merged`、`closed`、`unknown`。只有返回 `merged` 时框架才能把 request 标记为 `finished`。
- `tools/rebase-agent.sh`: 只处理 `SANDRONE_AGENT_PHASE=rebase`。它必须保留 base/master 新代码和 request 分支已通过 review 的实现语义，不能为了自己分支的修改删除 base/master 新代码；不得 commit、push、finish、approve/reject、创建 PR 或 merge。

`finish` 生成的 PR body 必须包含 `自动评审意见`，从最终 `reviews/<stage>/details/*.json` 汇总每个 reviewer 的 critical/high/warning/info finding。每条 finding 都应在 PR 描述里保留 title、evidence、impact、required_fix、suggested_change 和 verification，方便人类 reviewer 在 GitHub 或其他平台直接审查 warning/info，而不必回到本地 JSON。

`$SANDRONE_PLAN` 顶部必须保留 `## 规范化需求记录`，记录 request ID、external ID、source、URL、需求名称和需求描述。agent 可以重写计划正文，但不得删除或弱化这段记录。

CodeGraph 生命周期:

- `dev/repo/.codegraph` 是索引目录，供 CodeGraph MCP 查询目标仓库。框架会在非空 clone 和计划前检查中自动尝试初始化。
- `obsidian/codegraph/context.md` 是面向 agent/reviewer 的默认架构上下文，不等同于索引目录。decomposition/planning agent 和 reviewer 必须优先读取它，再决定是否用 CodeGraph MCP/CLI 深挖具体符号。
- CodeGraph CLI 不可用、索引缺失或 context 刷新失败时，不得假装已经分析；应在 preflight、journal 或 review finding 中记录风险，并告诉用户 `codegraph init -i dev/repo`、`codegraph context -p dev/repo <task>` 或 `SANDRONE_CODEGRAPH_BIN` 的恢复方式。

## Agent 要求

一个 request 会按 phase 被派发给 agent。planning agent 和 implementation agent 可以是同一个 connector 的不同提示词，也可以由你替换为不同后端；但 reviewer gate 必须由外层 `advance`/`tick` 执行，不得在子 Codex 里嵌套调用 reviewer。

所有 agent 必须:

- 读取 `$SANDRONE_REQUEST`、`$SANDRONE_DECOMPOSITION`/`dag.json`、`$SANDRONE_PLAN`、`$SANDRONE_CHANGE_DOC`、`$SANDRONE_AGENT_JOURNAL`、`status.json`、`obsidian/project.md`、`obsidian/codegraph/context.md`、`$SANDRONE_OBSIDIAN_NOTE` 和目标项目文档。
- decomposition 阶段只改拆解文档和 Obsidian 导航，不改目标代码，不运行 `submit`、`decomposition-review`、`plan-review`、`start`、`code-review`。
- decomposition agent 必须让拆解包含原始需求不变量、非目标、slice 列表、DAG、冲突域、小型需求覆盖说明、全局不变量、slice branch/完成状态/最终 PR 策略，并在退出前做 `DecompositionReviewer 提交前自检`。
- 保留并维护 `$SANDRONE_PLAN` 中的规范化需求记录，不得只根据标题写计划。
- 每一轮都必须向 `$SANDRONE_AGENT_JOURNAL` 记录读取内容、修改内容、review finding 处理、验证结果和下一步；每条 critical/high 必须有对应处理说明。
- planning 阶段只改 change 文档，不改目标代码，不运行 `submit`、`plan-review`、`start`、`code-review`。
- planning agent 必须让 plan 包含需求理解、目标依赖、仓库分析、目标项目内部要求、实现计划、测试验证、风险回滚和审批门禁。
- planning agent 退出前必须做 `PlanReviewer 提交前自检`，逐项核对 PlanReviewer 会审查的需求完整性、目标顺序、代码位置、测试策略、兼容/迁移/回滚、目标项目要求、硬编码/敏感信息和审批门禁。如果自检发现会产生 critical/high 的缺口，不得退出交给 reviewer，必须先修 `$SANDRONE_PLAN` 或 block，并把自检结果写入 `$SANDRONE_AGENT_JOURNAL`。
- plan-review 失败后的下一次 planning agent 必须读取 `reviews/plan-review/summary.json` 和最新 detail，逐条修复 `$SANDRONE_PLAN`。如果上一轮 summary 是 `gate_unavailable=true`，只作为历史诊断记录；恢复后不得仅凭旧 summary 再次 block，应修复当前产物并退出 0，让外层 `advance` 生成新的 review attempt。
- implementation 阶段只能在 `dev/worktrees/<request_id>` 中开发，不直接编辑 `dev/repo`。
- implementation agent 必须让 change-doc 包含实现前后对比、关键设计点、验证证据、目标项目要求完成情况和 reviewer finding 处理记录。
- implementation agent 完成开发后必须更新相关目标项目文档和 `$SANDRONE_CHANGE_DOC`。所有交付文档中的 checklist 必须全部打勾；无法由当前流程完成的事项不得保留为未勾选 checklist，必须移到 `后续流程`、`人工事项`、`阻塞项` 或同等章节，并写清 owner、触发条件、未完成原因和风险。不得把尚未真实完成的事项标成已完成，也不要为了凑勾篡改已批准 plan。
- implementation agent 运行测试时，如果发现不是由本分支改动导致的已有测试失败，也必须在当前 worktree 中修复并复验，不能用“不是本分支改的”作为忽略理由。除非修复会破坏 approved plan、需要外部权限/数据或无法安全判断，否则不得 block；修复必须在 `$SANDRONE_AGENT_JOURNAL` 和 `$SANDRONE_CHANGE_DOC` 的 Baseline failure 记录中写清失败命令、根因、修复范围和复验结果。
- implementation agent 不处理 PR rebase 冲突、PR outdated 或 base/master drift；这些属于 `sandrone pr-refresh` 和 RebaseAgent。发现此类问题时记录并 block 或等待外层支线，不得擅自 rebase、force push 或更新 PR。
- implementation agent 退出前必须做 `Code Review 提交前自检`，逐项核对 `tools/check-format.sh --format`、`tools/check-format.sh --check`、TestReviewer 的测试覆盖、失败路径、回归、baseline failure、验证证据，以及 DesignReviewer 的需求完成度、approved plan 符合度、可扩展性、硬编码、敏感信息、破坏性风险、错误处理、文档和 checklist。如果自检发现会产生 critical/high 的缺口，不得退出交给 code-review，必须先修复或 block，并把自检结果写入 `$SANDRONE_AGENT_JOURNAL` 和 `$SANDRONE_CHANGE_DOC`。
- code-review 失败后的下一次 implementation agent 必须先读取 `checks/format-check.md`（如果存在）和 TestReviewer/DesignReviewer 结果，修复格式、编译、clippy、代码、测试、文档和 change-doc。如果上一轮 summary 是 `gate_unavailable=true`，只作为历史诊断记录；恢复后不得仅凭旧 summary 再次 block，应修复当前产物并退出 0，让外层 `advance` 生成新的 code-review attempt。
- 如果 code-review summary 的 `recommended_next_phase` 为 `planning`，下一轮会回到 planning agent；这表示 approved plan 本身需要补目标、兼容、迁移、破坏性说明、测试策略或范围拆分。
- review attempt 默认按阶段区分: decomposition-review 最多 5 次，plan-review 最多 5 次，code-review 最多 20 次，integration-review 最多 20 次；超过后外层 advance/tick 会 block。`--max-attempts <n>` 只覆盖本次自动推进。agent 自己发现无法恢复时也必须运行 `sandrone block --request_id <REQ> --stage <decomposition|planning|implementation|rebase> --reason "<明确原因>"`。
- 不得 commit、push、创建 PR 或 merge。
- 不得调用 `sandrone submit`、`plan-review`、`code-review`、`start`、`finish`、`approve` 或 `reject`，不得手写、复制或修改 `status.json.gates` 或旧版 `approvals/*.approval.json` 来伪造审批。
- 不得修改 `tools/*review.sh`、`tools/schemas/*` 或新增本地/offline reviewer 来绕过模型 reviewer；如果本轮有新的、可直接验证的 reviewer backend 或网络失败证据，必须记录原因并 block。
- 每次 review 失败后必须先读取对应 `reviews/<stage>/summary.json`。历史 `gate_unavailable=true` 不能在 resume 后被当成当前 gate 仍不可用的证据；agent 不运行 reviewer，所以应记录该历史失败、修复当前 phase 产物并退出 0，由外层 `advance` 重新运行 reviewer。只有外层新一轮 reviewer 仍返回 `gate_unavailable=true` 时，框架才会再次 block。

## Runtime 文档

每个 request 的文档包:

```text
obsidian/changes/<name>/
  REQ-0001 index.md
  REQ-0001 request.md
  REQ-0001 decomposition.md
  decomposition.json
  dag.json
  REQ-0001 pr-doc.md
  REQ-0001 agent-journal.md
  status.json
  checks/
    format-check.md
  reviews/
```

`checks/format-check.md` 是 Obsidian change 包中的一等追踪文件，记录 code-review 前置格式/编译门禁的 stdout、stderr、exit code 和结论。slice 自己的 `<REQ-SNN> index.md` 和 `status.json` 使用本 slice 的 `checks/format-check.md`。

父 request 的 index 直接指向 `<REQ> agent-journal.md`、`<REQ> request.md`、`<REQ> decomposition.md` 和 `<REQ> pr-doc.md`；slice index 直接指向 `<REQ-SNN> agent-journal.md`、`<REQ-SNN> plan.md` 和 `<REQ-SNN> change-doc.md`。Agent Journal 不反向连接其他阶段文档，避免图谱过密。不要期待 runtime `spec.md`、`tasks.md`、`plan.html` 或旧版独立追踪文件。slice 的 `<REQ-SNN> plan.md` 是空白计划模板，也是 slice request；`<REQ-SNN> change-doc.md` 是简洁实现说明和证据导航，不复制完整 plan。review 原始 JSON 细节保留在:

```text
reviews/<stage>/details/
reviews/<stage>/summary.json
```

`sandrone upgrade` 会直接移除旧短文件名和当前模型不用的阶段文档，例如拆解父 request 中残留的父级 plan/change-doc、slice 中残留的 request/pr-doc。旧版 `approvals/` 会先迁移到 `status.json.gates`，再删除目录；agent 不应继续读写这些旧路径。

blocked 时会生成:

```text
<REQ> recovery.md
```

恢复入口:

```bash
sandrone resume --request_id <REQ-0001>
```

对于 `blocked` request，`resume` 必须真正恢复状态，而不是只打印路径。它会根据 plan gate 是否有效，把 request 恢复为 `planning` 或 `in-progress`，同步写回 `.sandrone/state/requests.tsv` 和 `obsidian/changes/<name>/status.json`。恢复后运行 `sandrone tick --request_id <REQ-0001>` 继续派发。

在 `start` 创建新 worktree 前，或者自动流程需要为 implementation phase 创建 worktree 前，框架必须先同步目标仓库基线。对非空且有 remote 的 `dev/repo` 运行 `git pull --ff-only`；如果能快进，就基于最新代码创建 worktree；如果 pull 失败、分叉或冲突，必须标记 request 为 `blocked` 并写入 recovery，不得创建过期或不一致的 worktree。

## 手动流程

自动流程之外，仍可手动运行:

```bash
sandrone update
sandrone decompose --name <YYYY-MM-DD-short-english-name> --request_id <REQ-0001>
sandrone submit --request_id <REQ-0001> --gate decomposition
sandrone decomposition-review --request_id <REQ-0001>
sandrone plan --name <YYYY-MM-DD-short-english-name> --request_id <REQ-0001>
sandrone submit --request_id <REQ-0001> --gate plan
sandrone plan-review --request_id <REQ-0001>
sandrone start --request_id <REQ-0001>
sandrone submit --request_id <REQ-0001> --gate change-doc
sandrone code-review --request_id <REQ-0001>
sandrone pr-refresh --request_id <REQ-0001>
sandrone obsidian-refresh
```

自动流程用 `sandrone tick` 发现和派发 request；hook 失败或需要手动推进单个 request 时，可以运行 `sandrone advance --request_id <REQ-0001>`。

审批是显式状态化门禁，不是口头约定。`status.json.gates` 记录 gate、来源和 `artifact_sha256`。如果审批后 `$SANDRONE_PLAN` 或 `$SANDRONE_CHANGE_DOC` 指向的阶段文档被修改，gate 会过期，必须重新提交和审批。

## Reviewer Gate

`plan-review` 调用 `PlanReviewer`。它必须基于需求标题、需求描述、目标仓库、CodeGraph、`$SANDRONE_REQUEST` 和 `$SANDRONE_PLAN` 审查计划。

`code-review` 必须先确认 plan gate 有效，然后调用:

- `TestReviewer`: 审查测试是否覆盖新增实现、失败路径、回归路径和目标项目要求；如果验证暴露不是由本分支改动导致的已有测试失败，还必须检查 implementation agent 是否修复并记录 Baseline failure。
- `DesignReviewer`: 审查实现是否满足需求和 approved plan，是否无硬编码、无隐私数据、无未授权破坏性变更、无明显 bug，并满足目标项目内部要求。

code-review 的 reviewer 必须相互独立。框架会为每个 reviewer 创建隔离 review context，只复制 request、plan、change-doc、status 和 `status.json.gates`，不复制 `reviews/`、summary/detail 或 agent journal。每个 reviewer 都必须基于一手证据重新评审，不得读取当前轮其他 reviewer 输出或历史 review 轮次。DesignReviewer 不能看 TestReviewer 的结论，也不能把 TestReviewer 的通过或拒绝当作自己的证据。

每个 reviewer 必须只输出一个 JSON 对象，不得输出 Markdown、代码块或解释性前后缀。字段必须包含 `reviewer`、`approved`、`gate_unavailable`、`decision`、`recommended_next_phase`、`summary`、`process`、`critical`、`high`、`warning` 和 `info`。任意 `critical/high`、`gate_unavailable=true` 或非法 JSON 都必须失败。

Finding 对象必须可执行，且必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。`evidence` 应指向具体文件、章节、命令、gate 状态、diff 或测试证据；`impact` 说明不修的风险；`required_fix` 说明通过 review 的必要条件；`suggested_change` 必须给出针对该条目的具体修改建议；`verification` 说明修完如何验证。默认 reviewer prompt 内置 approved、rejected 和 gate unavailable 的完整 JSON 示例；替换 reviewer backend 时必须保留同等严格的输出格式。

`recommended_next_phase` 规则:

- `planning`: 计划本身需要修改，或实现暴露出 approved plan 没覆盖的目标、迁移、兼容、破坏性风险或测试策略。
- `implementation`: 计划仍然有效，只需要修改代码、测试、change-doc 或验证证据。
- `blocked`: reviewer backend、关键文件、权限或上下文不可用，或者自动修复不安全。`gate_unavailable=true` 时必须用 `blocked`。

## Finish

用户确认 change-doc gate 后，才运行:

```bash
sandrone finish --request_id <REQ-0001> --message "feat: concise change summary"
```

`finish` 会在 request worktree 中 commit，push 到独立分支，生成包含关联需求、自动评审意见、request 文档和 change-doc 的 PR 描述，然后调用 `tools/pr-create.sh` 创建或复用 PR。PR 创建或复用成功后状态是 `wait-finish`，不是 `finished`；PR connector 失败时保持 `wait-update-pr`。它不会 merge。

PR 合入后运行:

```bash
sandrone pr-status --request_id <REQ-0001>
```

或者再次运行 `finish --request_id <REQ-0001>`。这两个入口都会调用 `tools/pr-status.sh`，只有脚本返回 `merged` 才标记 `finished`；如果 legacy `finished` 实际仍是 open PR，应修正为 `wait-finish`，如果脚本返回 missing/closed，则回到 `wait-update-pr` 等待重新创建或更新 PR。

如果 PR 已创建后 base/master 前进、平台提示冲突，或需要刷新 request 分支，运行:

```bash
sandrone pr-refresh --request_id <REQ-0001>
```

`pr-refresh` 会 fetch base、尝试 rebase，并在 clean rebase 后运行 IntegrationReviewer。发生冲突时，它会进入 `rebase-agent-running` 并派发 RebaseAgent；RebaseAgent 解决冲突后，外层 hook/advance 必须运行 IntegrationReviewer。IntegrationReviewer 是轻量集成门禁，不替代首次 TestReviewer + DesignReviewer；它重点审查:

每次真正发生 rebase 冲突时，框架必须写入 `pr-conflicts/attempts/NNN-rebase-conflict.md`，并在当前 request/slice 对应的 `$SANDRONE_CHANGE_DOC` 中追加 `PR 冲突记录`。同一个 PR 多次冲突时编号递增；非冲突刷新不得写冲突 attempt，避免污染审计历史。

- 冲突文件是否解决干净，没有 `<<<<<<<`、`=======`、`>>>>>>>` 残留。
- 是否保留 approved plan 和已通过 code-review 的实现语义。
- 是否只做集成适配，没有扩大需求范围。
- 是否处理了 base/master 新代码带来的接口、测试、配置或行为变化。
- 是否保留 base/master 新修改，不能为了自己分支的修改删除 base/master 新代码。
- 是否运行目标项目测试或合理替代验证。
- 对应 change-doc 是否记录冲突原因、解决方式、实现前后对比、base/master 保留证明和验证结果。

IntegrationReviewer 通过后会重新批准 `change-doc` 并回到 `wait-update-pr`。随后再次运行 `finish` 是对应的提交/推送脚本入口: 它会更新 PR body 并 push request 分支；如果没有新文件改动则跳过 commit，rebase 后非快进推送使用 `git push --force-with-lease -u origin <branch>`。任何 agent 都不得自行 merge。

如果冲突由人工或外部工具解决，可以运行 `sandrone pr-refresh --request_id <REQ-0001> --mode continue`，它会确认 rebase 已完成、没有 unmerged 文件，然后运行 IntegrationReviewer。

## 升级旧 Workspace

进入旧 workspace 后运行:

```bash
sandrone upgrade --dry-run
sandrone upgrade
```

`upgrade` 会补齐 schema、session registry、approval 目录、简化 runtime 文档、skill 副本，并把当前 workspace 写入全局 `workspaces.json`，也会把旧短文件名阶段文档迁移为带 request id 的 Obsidian 文件名。它不会覆盖 `dev/repo`、`dev/worktrees`、已填写的计划/变更文档、已有 `<REQ> agent-journal.md` 历史记录，也不会覆盖正式 `tools/*.sh`、`tools/prompts/*.md` 或 review schema。

`upgrade` 会刷新框架维护的 `.example.*` 参考文件，例如 `tools/issue-update.example.sh`、`tools/issue-agent.example.sh`、`tools/check-format.example.sh`、`tools/plan-review.example.sh`、`tools/prompts/plan-reviewer.example.md` 和 `tools/schemas/review-result.example.schema.json`。这些文件用于比较新版默认实现、测试 connector 或手动复制到正式脚本；不要把用户本地定制直接写在 `.example.*` 里。

如果确认当前 workspace 没有自定义 connector、prompt 或 schema，或者已经人工确认要全部回到框架默认实现，可以运行:

```bash
sandrone upgrade --default
```

`--default` 会先刷新 `.example.*`，再把这些 example 覆盖到对应正式文件。普通 `upgrade` 的输出会提醒用户自行决定替换哪些脚本。
