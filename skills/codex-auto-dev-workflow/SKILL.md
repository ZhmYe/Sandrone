---
name: codex-auto-dev-workflow
description: Use when the user asks Codex to create, clone, update, tick, plan, implement, review, block, resume, finish, upgrade, approve, or manage software work with codex-auto-dev workspaces, especially when explicit approval gates, request IDs, Chinese change templates, isolated worktrees, issue-agent automation, recovery docs, target project checks, no-commit/no-push agent boundaries, or finish-time PR delivery matter.
metadata:
  short-description: Run codex-auto-dev approval-gated workspaces
---

# Codex Auto Dev Workflow

当用户要求 Codex 新建仓库、clone 仓库、同步需求、自动处理 issue、写计划、实现需求、审批、阻塞恢复、升级旧 workspace 或完成变更时，使用这个 skill。

## 必做第一步: 安装或验证 CLI

Before any workspace command, verify that the CLI is installed:

```bash
codex-auto-dev --help
```

如果命令不存在，先停止并告诉用户这个 skill 还需要 Rust CLI。只有在用户明确批准后，才可以安装 CLI 和本地 skill:

```bash
curl -fsSL https://raw.githubusercontent.com/ZhmYe/codex-auto-dev-workflow/master/scripts/bootstrap.sh | sh
```

如果当前已经 clone 了本仓库，可以用:

```bash
scripts/install.sh --force
```

Do not run workspace commands until `codex-auto-dev --help` succeeds.

## 核心边界

`codex-auto-dev` 只做机械动作: 创建 workspace、clone 目标仓库、request 记录、简洁 change 文档包、approval 文件、review 结果、session registry、worktree、blocked/recovery 和 finish-time commit/push/PR。

Codex CLI 子运行负责思考和交付: 填写 `plan.md`、实现代码、运行目标项目检查、写 `change-doc.md`、根据 reviewer 结果修复、记录 `agent-journal.md`。

自动化无人值守流程必须通过 reviewer gate 推进，不得直接跳过审批。默认 connector 都可替换:

- `tools/issue-update.sh`
- `tools/issue-agent.sh`
- `tools/plan-review.sh`
- `tools/test-review.sh`
- `tools/design-review.sh`
- `tools/pr-create.sh`

## 自动 Heartbeat 流程

主 session 或 heartbeat 应调用:

```bash
codex-auto-dev tick
```

`tick` 做短主控和兜底恢复:

1. 运行 `update`。
2. 刷新已结束 agent 的状态。
3. 找出 eligible request；如果传了 `--request_id`，只处理该 request。
4. 必要时为每个 request 创建 change 文档包。
5. 在并发上限内，对需要计划或实现的 request，异步启动对应 agent。
6. 对漏掉 hook 的已结束 agent，执行同 `advance` 一样的 gate 推进。
7. code-review 通过后标记 `waiting-finish`；tick 永远不运行 `finish`。

新 workspace 的 `.codex-auto-dev/config.toml` 默认包含 `parallel_limit = 1`，也就是同一时间最多自动处理 1 个 issue。`tick` 会统计当前 `planning-agent-running`、`implementation-agent-running` 和 legacy `agent-running` request，只有剩余槽位才会派发新的 request。需要并行处理多个 issue 时，可以修改配置，或单次运行:

```bash
codex-auto-dev tick --parallel-limit 2
```

运行中的 request 保持 `planning-agent-running` 或 `implementation-agent-running`，不会重复派发。agent stdout、stderr、pid、exit code 和 hook log 写入 `.codex-auto-dev/state/agents/`。agent wrapper 写入 exit code 后会立即调用 `codex-auto-dev advance --request_id <REQ>`；因此正常情况下不需要等下一次 heartbeat 才 review。需要定时时，让 Codex heartbeat、cron 或其他调度器每 15 分钟调用一次 `codex-auto-dev tick` 发现新需求和兜底恢复。

`advance` 是单 request 推进器:

```bash
codex-auto-dev advance --request_id <REQ-0001>
```

它不运行 issue update，不扫描全部 request，只在 per-request lock 下刷新一个 request、提交 gate、执行 reviewer、创建 worktree、派发下一 phase 或标记 `waiting-finish`/`blocked`。hook 和 heartbeat 同时触发时，拿不到 `.codex-auto-dev/state/locks/<request_id>.lock/` 的一方会跳过。

运行环境或 reviewer 可用性不确定时，先运行:

```bash
codex-auto-dev doctor
```

`doctor` 检查 workspace、Git、Codex CLI、GitHub CLI、CodeGraph CLI、target repo、agent/reviewer connector、review schema、CodeGraph index 和事件流目录。它显示 warning/fail，不得 panic。

所有关键状态变化都会追加到 `.codex-auto-dev/state/events.ndjson`。该文件是审计、前端展示和恢复分析的稳定事件流；不要让 agent 手动改写它。

对非空目标仓库，`new --url` 和计划前检查会自动尝试运行 `codegraph init -i dev/repo`，让 CodeGraph MCP 能读取目标仓库索引。如果 CodeGraph CLI 不存在或初始化失败，流程只记录 warning，不得 panic。`docs/codegraph/context.md` 是给 agent/reviewer 阅读的架构文档，仍需要通过 `codegraph-project-preview` skill 生成或刷新。

## Connector Contract

所有可替换脚本都必须遵守稳定输入输出契约，保证 issue-agent prompt 可以保持通用，不依赖 GitHub/Jira/内部系统的特定字段。

- `tools/issue-update.sh`: stdout 输出零行或多行 TSV，无 header。字段必须是 `external_id<TAB>source<TAB>title<TAB>body<TAB>url`。`external_id` 必须稳定，`source` 是短平台名，`title` 是规范化需求名称，`body` 是完整需求描述，`url` 可为空。
- `tools/issue-agent.sh`: 输入来自 `CODEX_AUTO_DEV_*` 环境变量和 runtime 文档。`CODEX_AUTO_DEV_AGENT_PHASE=planning` 时只写 `plan.md`；`implementation` 时只在 `CODEX_AUTO_DEV_WORKTREE` 写代码并更新 `change-doc.md`。成功/失败由退出码表示，失败时 stderr 必须给出可恢复原因。默认 agent/reviewer connector 不写死 Codex.app 路径；需要从普通终端运行时，可以把 `codex` 放进 `PATH`，或设置 `CODEX_AUTO_DEV_CODEX_BIN` 指向可执行文件，或设置 `CODEX_AUTO_DEV_CODEX_APP` 指向 Codex app bundle。
- `tools/plan-review.sh`、`tools/test-review.sh`、`tools/design-review.sh`: stdout 必须是一个符合 `tools/schemas/review-result.schema.json` 的 JSON 对象。非法 JSON、空输出或脚本失败都会变成 `gate_unavailable=true` 的 blocking review；自定义 reviewer 如果无法可靠评审，也必须返回 `gate_unavailable=true` 或非 0 退出。每个 reviewer 必须返回 `recommended_next_phase`，只能是 `planning`、`implementation` 或 `blocked`。
- reviewer 输入必须来自隔离的 `$CODEX_AUTO_DEV_REVIEW_CONTEXT`，其中只包含 request、plan、change-doc、status 和 approvals，不包含 `reviews/` 或 agent journal。code-review 中 TestReviewer 与 DesignReviewer 必须独立重新评审，不得读取其他 reviewer 输出、历史 summary/detail、上一轮 review 意见或 `$CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS`；DesignReviewer 不得依赖 TestReviewer 结论。
- `tools/pr-create.sh`: 必须先判断当前平台/仓库是否支持创建 PR，再检查 base/head 是否已经存在 PR。成功时 stdout 输出一个 TSV 行: `created<TAB>url` 或 `existing<TAB>url`；旧脚本只输出 URL 仍按 created 兼容。失败时 stderr 输出原因。它不得 merge。

`finish` 生成的 PR body 必须包含 `自动评审意见`，从最终 `reviews/<stage>/details/*.json` 汇总每个 reviewer 的 critical/high/warning/info finding。每条 finding 都应在 PR 描述里保留 title、evidence、impact、required_fix、suggested_change 和 verification，方便人类 reviewer 在 GitHub 或其他平台直接审查 warning/info，而不必回到本地 JSON。

`plan.md` 顶部必须保留 `## 规范化需求记录`，记录 request ID、external ID、source、URL、需求名称和需求描述。agent 可以重写计划正文，但不得删除或弱化这段记录。

CodeGraph 生命周期:

- `dev/repo/.codegraph` 是索引目录，供 CodeGraph MCP 查询目标仓库。框架会在非空 clone 和计划前检查中自动尝试初始化。
- `docs/codegraph/context.md` 是面向 agent/reviewer 的架构文档，不等同于索引目录。planning agent 和 reviewer 应读取它；如果缺失或过期，先运行 `codegraph-project-preview` skill 生成或刷新。
- CodeGraph CLI 不可用时，不得跳过需求分析；应在 plan preflight、journal 或 review finding 中记录风险。

## Agent 要求

一个 request 会按 phase 被派发给 agent。planning agent 和 implementation agent 可以是同一个 connector 的不同提示词，也可以由你替换为不同后端；但 reviewer gate 必须由外层 `advance`/`tick` 执行，不得在子 Codex 里嵌套调用 reviewer。

所有 agent 必须:

- 读取 `request.md`、`plan.md`、`change-doc.md`、`agent-journal.md`、`status.json` 和目标项目文档。
- 保留并维护 `plan.md` 中的规范化需求记录，不得只根据标题写计划。
- 每一轮都必须向 `agent-journal.md` 记录读取内容、修改内容、review finding 处理、验证结果和下一步；每条 critical/high 必须有对应处理说明。
- planning 阶段只改 change 文档，不改目标代码，不运行 `submit`、`plan-review`、`start`、`code-review`。
- planning agent 必须让 plan 包含需求理解、目标依赖、仓库分析、目标项目内部要求、实现计划、测试验证、风险回滚和审批门禁。
- plan-review 失败后的下一次 planning agent 必须读取 `reviews/plan-review/summary.json` 和最新 detail，逐条修复 `plan.md`。
- implementation 阶段只能在 `dev/worktrees/<request_id>` 中开发，不直接编辑 `dev/repo`。
- implementation agent 必须让 change-doc 包含实现前后对比、关键设计点、验证证据、目标项目要求完成情况和 reviewer finding 处理记录。
- implementation agent 完成开发后必须更新相关目标项目文档和 `change-doc.md`。所有交付文档中的 checklist 必须全部打勾；无法由当前流程完成的事项不得保留为未勾选 checklist，必须移到 `后续流程`、`人工事项`、`阻塞项` 或同等章节，并写清 owner、触发条件、未完成原因和风险。不得把尚未真实完成的事项标成已完成，也不要为了凑勾篡改已批准 plan。
- implementation agent 运行测试时，如果发现不是由本分支改动导致的已有测试失败，也必须在当前 worktree 中修复并复验，不能用“不是本分支改的”作为忽略理由。除非修复会破坏 approved plan、需要外部权限/数据或无法安全判断，否则不得 block；修复必须在 `agent-journal.md` 和 `change-doc.md` 的 Baseline failure 记录中写清失败命令、根因、修复范围和复验结果。
- code-review 失败后的下一次 implementation agent 必须同时读取 TestReviewer 和 DesignReviewer 结果，修复代码、测试、文档和 change-doc。
- 如果 code-review summary 的 `recommended_next_phase` 为 `planning`，下一轮会回到 planning agent；这表示 approved plan 本身需要补目标、兼容、迁移、破坏性说明、测试策略或范围拆分。
- 默认最多 20 个 review attempt；超过后外层 advance/tick 会 block。agent 自己发现无法恢复时也必须运行 `codex-auto-dev block --request_id <REQ> --stage <planning|implementation> --reason "<明确原因>"`。
- 不得 commit、push、创建 PR 或 merge。
- 不得调用 `codex-auto-dev submit`、`plan-review`、`code-review`、`start`、`finish`、`approve` 或 `reject`，不得手写、复制或修改 `approvals/*.approval.json` 来伪造审批。
- 不得修改 `tools/*review.sh`、`tools/schemas/*` 或新增本地/offline reviewer 来绕过模型 reviewer；如果 reviewer backend 或网络失败，必须记录原因并 block。
- 每次 review 失败后必须先读取对应 `reviews/<stage>/summary.json`。如果任一 reviewer 的 `gate_unavailable` 为 `true`，必须立即 `block`，不得重试、不得改 reviewer、不得手动 approve。

## Runtime 文档

每个 request 的文档包:

```text
docs/changes/<name>/
  request.md
  plan.md
  change-doc.md
  agent-journal.md
  status.json
  approvals/
  reviews/
```

不要期待 runtime `spec.md`、`tasks.md` 或 `plan.html`。`plan.md` 合并规格、计划和任务清单。`change-doc.md` 必须包含最终 review 结果摘要，review 原始 JSON 细节保留在:

```text
reviews/<stage>/details/
reviews/<stage>/summary.json
```

blocked 时会生成:

```text
recovery.md
```

恢复入口:

```bash
codex-auto-dev resume --request_id <REQ-0001>
```

对于 `blocked` request，`resume` 必须真正恢复状态，而不是只打印路径。它会根据 plan approval 是否有效，把 request 恢复为 `planning` 或 `in-progress`，同步写回 `.codex-auto-dev/state/requests.tsv` 和 `docs/changes/<name>/status.json`。恢复后运行 `codex-auto-dev tick --request_id <REQ-0001>` 继续派发。

在 `start` 创建新 worktree 前，或者自动流程需要为 implementation phase 创建 worktree 前，框架必须先同步目标仓库基线。对非空且有 remote 的 `dev/repo` 运行 `git pull --ff-only`；如果能快进，就基于最新代码创建 worktree；如果 pull 失败、分叉或冲突，必须标记 request 为 `blocked` 并写入 recovery，不得创建过期或不一致的 worktree。

## 手动流程

自动流程之外，仍可手动运行:

```bash
codex-auto-dev update
codex-auto-dev plan --name <YYYY-MM-DD-short-english-name> --request_id <REQ-0001>
codex-auto-dev submit --request_id <REQ-0001> --gate plan
codex-auto-dev plan-review --request_id <REQ-0001>
codex-auto-dev start --request_id <REQ-0001>
codex-auto-dev submit --request_id <REQ-0001> --gate change-doc
codex-auto-dev code-review --request_id <REQ-0001>
```

自动流程用 `codex-auto-dev tick` 发现和派发 request；hook 失败或需要手动推进单个 request 时，可以运行 `codex-auto-dev advance --request_id <REQ-0001>`。

审批是显式文件化门禁，不是口头约定。approval 文件记录 `artifact_sha256`。如果审批后 `plan.md` 或 `change-doc.md` 被修改，approval 会过期，必须重新提交和审批。

## Reviewer Gate

`plan-review` 调用 `PlanReviewer`。它必须基于需求标题、需求描述、目标仓库、CodeGraph、`request.md` 和 `plan.md` 审查计划。

`code-review` 必须先确认 plan approval 有效，然后调用:

- `TestReviewer`: 审查测试是否覆盖新增实现、失败路径、回归路径和目标项目要求；如果验证暴露不是由本分支改动导致的已有测试失败，还必须检查 implementation agent 是否修复并记录 Baseline failure。
- `DesignReviewer`: 审查实现是否满足需求和 approved plan，是否无硬编码、无隐私数据、无未授权破坏性变更、无明显 bug，并满足目标项目内部要求。

code-review 的 reviewer 必须相互独立。框架会为每个 reviewer 创建隔离 review context，只复制 request、plan、change-doc、status 和 approvals，不复制 `reviews/`、summary/detail 或 agent journal。每个 reviewer 都必须基于一手证据重新评审，不得读取当前轮其他 reviewer 输出或历史 review 轮次。DesignReviewer 不能看 TestReviewer 的结论，也不能把 TestReviewer 的通过或拒绝当作自己的证据。

每个 reviewer 必须只输出一个 JSON 对象，不得输出 Markdown、代码块或解释性前后缀。字段必须包含 `reviewer`、`approved`、`gate_unavailable`、`decision`、`recommended_next_phase`、`summary`、`process`、`critical`、`high`、`warning` 和 `info`。任意 `critical/high`、`gate_unavailable=true` 或非法 JSON 都必须失败。

Finding 对象必须可执行，且必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。`evidence` 应指向具体文件、章节、命令、approval、diff 或测试证据；`impact` 说明不修的风险；`required_fix` 说明通过 review 的必要条件；`suggested_change` 必须给出针对该条目的具体修改建议；`verification` 说明修完如何验证。默认 reviewer prompt 内置 approved、rejected 和 gate unavailable 的完整 JSON 示例；替换 reviewer backend 时必须保留同等严格的输出格式。

`recommended_next_phase` 规则:

- `planning`: 计划本身需要修改，或实现暴露出 approved plan 没覆盖的目标、迁移、兼容、破坏性风险或测试策略。
- `implementation`: 计划仍然有效，只需要修改代码、测试、change-doc 或验证证据。
- `blocked`: reviewer backend、关键文件、权限或上下文不可用，或者自动修复不安全。`gate_unavailable=true` 时必须用 `blocked`。

## Finish

用户确认 change-doc approval 后，才运行:

```bash
codex-auto-dev finish --request_id <REQ-0001> --message "feat: concise change summary"
```

`finish` 会在 request worktree 中 commit，push 到独立分支，生成包含关联需求、自动评审意见、request 文档和 change-doc 的 PR 描述，然后调用 `tools/pr-create.sh` 创建或复用 PR。它不会 merge。

## 升级旧 Workspace

进入旧 workspace 后运行:

```bash
codex-auto-dev upgrade --dry-run
codex-auto-dev upgrade
```

`upgrade` 会补齐 schema、session registry、approval 目录、简化 runtime 文档和 skill 副本。它不会覆盖 `dev/repo`、`dev/worktrees`、已填写的计划/变更文档，也不会覆盖正式 `tools/*.sh`、`tools/prompts/*.md` 或 review schema。

`upgrade` 会刷新框架维护的 `.example.*` 参考文件，例如 `tools/issue-update.example.sh`、`tools/issue-agent.example.sh`、`tools/plan-review.example.sh`、`tools/prompts/plan-reviewer.example.md` 和 `tools/schemas/review-result.example.schema.json`。这些文件用于比较新版默认实现、测试 connector 或手动复制到正式脚本；不要把用户本地定制直接写在 `.example.*` 里。

如果确认当前 workspace 没有自定义 connector、prompt 或 schema，或者已经人工确认要全部回到框架默认实现，可以运行:

```bash
codex-auto-dev upgrade --default
```

`--default` 会先刷新 `.example.*`，再把这些 example 覆盖到对应正式文件。普通 `upgrade` 的输出会提醒用户自行决定替换哪些脚本。
