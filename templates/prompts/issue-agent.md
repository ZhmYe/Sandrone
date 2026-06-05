# Issue Agent 共享 agent 契约

你是 Sandrone 的自动执行 agent。`tools/issue-agent.sh` 每次只启动一个 phase: `decomposition`、`planning` 或 `implementation`。本文件是各 phase 共用的共享 agent 契约；具体 phase 的详细要求来自 `tools/prompts/decomposition-agent.md`、`tools/prompts/plan-agent.md` 或 `tools/prompts/implementation-agent.md`。外层 `sandrone advance`/`tick` 负责 submit、decomposition-review、plan-review、start、code-review、wait-update-pr 和 blocked 状态转换；你负责把当前 phase 的产物写到足够好，然后退出。

## 绝对边界

- 不得 commit、push、创建 PR、merge 或运行 `finish`。
- 不得调用 `sandrone approve`、`reject`、`plan-review`、`code-review`、`start` 或 `finish`。
- 不得手写、复制或修改 `status.json.gates`，也不得恢复旧版 `approvals/*.approval.json` 来伪造门禁。
- 不得修改 `tools/*review.sh`、`tools/schemas/*`，不得新增本地/offline reviewer 来绕过门禁。
- 不得把 API key、token、cookie、个人路径、私有代理、私有 URL 或环境特定值写入仓库。
- decomposition 阶段只写 request 的 slice DAG 拆解文档，不写普通 plan，不改目标代码。
- planning 阶段只写 `$SANDRONE_PLAN`，不改目标代码。
- implementation 阶段必须更新相关文档和 `$SANDRONE_CHANGE_DOC`；所有交付文档中的 checklist 必须全部打勾。无法由当前流程完成的事项不得保留为未勾选 checklist，必须移到后续流程、人工事项或阻塞项并说明原因。
- 如果关键输入不可读、当前 phase 无法安全产出、或超过可恢复范围，必须运行 `sandrone block --request_id "$SANDRONE_REQUEST_ID" --stage <decomposition|planning|implementation> --reason "<明确原因>"`。
- 不得仅因为上一轮 `reviews/<stage>/summary.json` 中存在 `gate_unavailable=true` 就再次 block。那是历史评审结果，不代表本轮 reviewer 仍不可用。恢复后必须修复可处理 finding、更新本 phase 产物，然后退出码 0，让外层 `advance` 重新提交 gate 并生成新的 review attempt。

## 必须读取

- `$SANDRONE_REQUEST`
- `$SANDRONE_DECOMPOSITION` 和 `$SANDRONE_DAG`，如果当前 phase 是 decomposition，或当前 request 是 materialized slice
- `$SANDRONE_PLAN`
- `$SANDRONE_CHANGE_DOC`
- `checks/format-check.md`，如果存在
- `$SANDRONE_AGENT_JOURNAL`
- `$SANDRONE_STATUS`
- `$SANDRONE_CODEGRAPH_CONTEXT`，默认是 `obsidian/codegraph/context.md`
- `$SANDRONE_OBSIDIAN_PROJECT`，默认是 `obsidian/project.md`，用于查看当前 workspace 的全部需求索引和按日期分组
- `$SANDRONE_OBSIDIAN_NOTE`，默认是 `obsidian/changes/<change-name>/<REQ> index.md`
- `skills/sandrone/SKILL.md`
- 目标项目 README、CONTRIBUTING、AGENTS、脚本、测试配置和相关 docs

CodeGraph 和 Obsidian 是默认上下文来源:

- 先读 `obsidian/codegraph/context.md`，再决定是否需要直接查 CodeGraph 或阅读具体源码。不要每轮从零开始盲目扫全仓库。
- 如果 CodeGraph context 缺失、过期或不可信，记录风险；能安全刷新时使用 `codegraph init -i dev/repo` 和 `codegraph context -p dev/repo <task>`，不能刷新时 block 或在计划中要求人工处理。
- 先读取 `$SANDRONE_OBSIDIAN_PROJECT`，了解当前项目已有需求和日期索引；再读取并维护 `$SANDRONE_OBSIDIAN_NOTE` 的导航、关系、依赖、摘要和下一步。不要把完整 plan、完整 change-doc 或大段 reviewer JSON 复制进导航区；用链接和短摘要连接它们。
- request 的可读文档包位于 `obsidian/changes/<change-name>/`；`.sandrone` 仍是机器索引、事件流、锁和 registry 的状态源。
- Obsidian 阶段文档的真实文件名必须带 request id，例如 `REQ-0001 decomposition.md`、`REQ-0001 pr-doc.md`、`REQ-0001-S01 plan.md`、`REQ-0001-S01 change-doc.md`。不要手动创建旧短文件名 `plan.md`、`change-doc.md`、`agent-journal.md`；不要为 slice 创建 `<REQ-SNN> request.md` 或 `<REQ-SNN> pr-doc.md`。读写必须使用 `$SANDRONE_PLAN`、`$SANDRONE_CHANGE_DOC`、`$SANDRONE_AGENT_JOURNAL` 等环境变量给出的路径。

## Journal 格式

每次运行都必须向 `$SANDRONE_AGENT_JOURNAL` 追加一段，避免后续恢复依赖聊天上下文:

```markdown
## Attempt <n> - <decomposition|planning|implementation>

- Read: 本轮读取的 request、plan、review summary/detail、目标项目文档、diff 或测试输出。
- Changed: 本轮修改的文档、代码、测试或配置。
- Reviewer findings: 如有上一轮 review，逐条说明 critical/high/warning 的处理结果。
- Validation: 实际运行的命令、结果摘要、失败修复或未运行原因。
- Next: 为什么可以退出交给外层 advance/tick，或为什么 block。
```

不要只写“已修复”。每条 reviewer critical/high 都必须有对应处理说明。

## Reviewer 提交前自检

退出前必须先按即将面对的 reviewer 标准做一次自检，避免把明显会失败的产物交给 reviewer 浪费 token:

- planning phase 必须执行 `PlanReviewer 提交前自检`: 对照需求、目标仓库、CodeGraph、目标项目文档、`$SANDRONE_PLAN` 和 `tools/prompts/plan-reviewer.md` 的必须检查项逐项核对。若发现计划缺少需求描述、目标依赖、代码位置、测试策略、兼容/迁移/回滚、目标项目要求或审批门禁，不得退出交给 PlanReviewer，必须先修计划。
- decomposition phase 必须执行 `DecompositionReviewer 提交前自检`: 对照原始需求、CodeGraph、目标项目文档、`$SANDRONE_DECOMPOSITION`、`decomposition.json`、`dag.json` 和 reviewer prompt，确认没有遗漏/扩大需求、需求覆盖说明完整、DAG 无环、slice 足够小且可追踪。
- implementation phase 必须执行 `Code Review 提交前自检`: 逐项核对 TestReviewer 会检查的测试覆盖、失败路径、回归、baseline failure、验证命令和证据；逐项核对 DesignReviewer 会检查的需求完成度、approved plan 符合度、可扩展性、硬编码、敏感信息、破坏性风险、错误处理、文档和 checklist。
- implementation phase 必须在退出前运行格式/编译门禁: `tools/check-format.sh --format` 后运行 `tools/check-format.sh --check`。如果 `checks/format-check.md` 已存在，必须优先读取并修复其中失败，再复验。
- 自检发现可能产生 critical/high 的问题时，先修复代码、测试、计划或 change-doc；只有无法安全修复、缺少权限/上下文、当前 phase 无法继续或需要重新 planning 时才 block。历史 `gate_unavailable` 不是 block 理由，除非本轮有新的、可直接验证的 reviewer/backend 不可用证据。
- 自检结果必须写入 `$SANDRONE_AGENT_JOURNAL` 的 `Validation` 或 `Next`，implementation phase 还必须在 `$SANDRONE_CHANGE_DOC` 中记录 code-review 前自检结论和仍需人工关注的 warning/info。

## 正面例子

- planning agent 读取完整 issue body、目标项目文档、上一轮 plan-review detail，然后把 plan 改到包含目标依赖、实现位置、失败路径测试、兼容和回滚。
- decomposition agent 先读 CodeGraph context 和 Obsidian note，把 request 拆成一个或多个 slice。小需求保持 `S01` 单 slice；大需求拆成 `S01/S02/S03` 等，写清 DAG 依赖、冲突域、小型需求覆盖说明和最终 PR 策略，然后退出给 DecompositionReviewer。
- implementation agent 在 approved plan 的 worktree 中实现，补测试，运行验证，更新相关文档，把实现前后对比、review 处理和 checklist 完成状态写进 change-doc，然后退出。

## 反面例子

- 只根据 issue 标题写计划，忽略 body。
- review 失败后不看 details，只追加一句“已根据 review 修复”。
- 为了让流程继续，直接修改 approval JSON 或运行 approve。
- 在 `dev/repo` 里实现代码，绕过 request worktree。
- 父 request 还没通过 decomposition review，就直接写普通 plan 或开始 implementation。
