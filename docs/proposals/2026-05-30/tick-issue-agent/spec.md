# 规格: Tick Issue Agent

## 背景

用户希望 heartbeat 主 session 只负责扫描和派发，不亲自写计划、写代码或做长循环。每个 issue 应由一个 Codex CLI 子 session 连续处理 planning 和 implementation，这样上下文保持完整；同时通过详细文档、review 结果和状态文件避免上下文过长后无法恢复。

## 用户目标

新增 `codex-auto-dev tick`。一次 tick 先更新 issue，再刷新已结束 agent 状态，然后为所有可处理 request 生成简洁 change 文档包，并分别调用 `tools/issue-agent.sh` 异步启动 Codex CLI 子运行。每个子运行负责从 plan 写到 implementation，并在内部根据 `plan-review` / `code-review` 最多修复 20 轮。自动流程默认停在 change-doc approval 通过后，等待用户决定是否 `finish`。

## 功能要求

- 新增 `codex-auto-dev tick`。
- 支持 `--request_id <REQ>` 精确派发一个 request。
- 支持 `--max-attempts <N>`，默认 `20`，传给 issue agent。
- tick 必须先运行 issue update。
- tick 默认派发全部 eligible request，不等待 issue-agent 结束。
- 支持通过 agent 状态文件记录 pid、stdout、stderr 和 exit code。
- 运行中的 request 不重复派发；后续 tick 根据 approval 或 exit code 刷新为 `waiting-finish` 或 `blocked`。
- 新增可替换 issue agent connector:
  - `tools/issue-agent.sh`
  - `tools/prompts/issue-agent.md`
- runtime change 文档包必须简化为:
  - `request.md`
  - `plan.md`
  - `change-doc.md`
  - `agent-journal.md`
  - `status.json`
  - `approvals/`
  - `reviews/`
- runtime 不再生成 `spec.md`、`tasks.md`、`plan.html`、`codex-plan.md`、`codex-start.md`。
- `plan.md` 合并规格、计划和任务清单。
- `plan.md` 顶部必须包含平台无关的规范化需求记录，包括 request ID、external ID、source、URL、需求名称和需求描述。
- `change-doc.md` 必须包含最终 review 结果摘要。
- 默认 connector 必须写明稳定输入输出契约，保证 issue-agent prompt 不依赖某个平台的字段格式。
- review JSON 细节放到 `reviews/<stage>/details/`，summary 放到 `reviews/<stage>/summary.json`。
- 超过最大 review 修复轮数时，issue agent 必须调用 `codex-auto-dev block` 标记阻塞，并生成 `recovery.md`。
- 新增 `codex-auto-dev resume --request_id <REQ>`，用于快速输出恢复入口。

## 非目标

- 不自动 merge。
- 默认不自动 finish。
- 不实现跨 issue 冲突自动合并。
- 不让 Rust CLI 自己写计划或代码。

## 验收标准

- 新 workspace 默认包含 `tools/issue-agent.sh` 和 `tools/prompts/issue-agent.md`。
- `plan` 只生成简洁 runtime 文档包，不生成 `spec.md`、`tasks.md` 或 `plan.html`。
- `plan.md` 包含规范化需求记录，且记录完整需求描述。
- 默认可替换脚本包含 Connector contract。
- `tick` 会 update、生成 change packet、批量异步调用 issue-agent，并在后续 tick 发现 change-doc approval 通过后标记 `waiting-finish`。
- `block` 会写 `status.json`、`recovery.md`，并把 request 状态标为 `blocked`。
- `resume` 输出 `recovery.md`、worktree、branch、关键文档路径和下一步命令。
- code-review 每轮详情存入 details，最终摘要同步进 `change-doc.md`。
