# 规格: Explicit Approval, Thread Registry And Upgrade

## 背景

现有流程依赖 Codex 在对话里“停下来”等待用户批准。这个约束对单线程手工试用可行，但不适合机器人、定时任务和多个需求并行开发。旧 workspace 也缺少升级路径，一旦框架模板、skill 或状态结构变化，已经创建的项目无法同步获得新能力。

## 用户目标

用户希望 `sandrone` 把审批变成显式、可审计、可机器读取的状态文件；把每个 request 的 planning / implementation thread 记录下来；并提供 `upgrade` 命令，让旧 workspace 补齐新的 approval、session 和中文模板结构。

## 功能要求

- `submit --request_id <id> --gate <plan|change-doc>` 创建或刷新 approval 文件，并记录当前 artifact hash。
- `approve/reject --request_id <id> --gate <gate> --by <actor>` 写入审批决定。
- `approvals --request_id <id> --json` 输出可给机器人读取的审批状态。
- `start` 必须检查 `plan` approval 已批准且 artifact 未变更。
- `finish` 必须检查 `change-doc` approval 已批准且 artifact 未变更。
- approval 文件必须包含 `artifact_sha256`，审批后文档被改动时，后续命令必须报 stale。
- `session` 命令允许登记 Codex thread ID / URL / status。
- `sessions --json` 输出 `.sandrone/sessions.json`，供后续前端或机器人展示。
- `upgrade --dry-run` 只显示将执行的迁移。
- `upgrade` 补齐 schema version、session registry、approval 目录、最新 skill 和中文模板。
- `upgrade` 不得覆盖目标仓库、worktree、用户改过的 issue connector，以及已填写的计划或变更文档。
- `change-doc.md` 模板必须强调实现说明、实现前后对比和关键设计点，不要求完整列出所有文件变更。

## 文档语言要求

面向人的 runtime 文档默认使用中文。命令、路径、JSON/TOML key、结构体名称、状态枚举、测试输出和外部原文保持原样。目标项目要求英文文档时，按目标项目要求执行，并在框架 change doc 中用中文说明原因。

## 非目标

- 不在本次实现中自动创建 Codex App thread。
- 不在本次实现中自动创建 GitHub PR。
- 不引入前端界面。
- 不引入外部 Rust crate；第一版保持标准库实现，降低安装阻力。

## 验收标准

- 未批准 plan 时运行 `start` 必须失败。
- 批准 plan 后修改 `plan.md`，运行 `start` 必须失败并提示 approval stale。
- 未批准 change doc 时运行 `finish` 必须失败。
- session registry 能登记并输出 thread URL。
- `upgrade` 能迁移旧 workspace，并保留用户自定义的 `tools/issue-update.sh`。
- 生成模板和 handoff 默认使用中文，并包含目标仓库、原始需求、approval 文件和新 thread 要求。
- `change-doc.md` 包含实现前后对比、关键设计点和变更范围摘要。
