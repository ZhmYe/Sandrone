# Spec: Comprehensive Chinese README

## 背景

框架已经从最初的 Rust CLI 原型演进为包含自动 tick、outer advance、agent wrapper hook、严格 reviewer gate、独立 worktree、finish-time PR connector、CodeGraph 初始化、环境变量配置、旧 workspace upgrade 和可恢复状态机的完整自动开发外框架。原 README 虽然已有中文内容，但结构更像阶段性说明，缺少完整用户手册所需的项目定位、可视化流程、命令分组、配置、环境、恢复和注意事项。

## 目标

- 用中文重写 README，使其成为用户入门、配置、运行和排障的主要入口。
- 明确项目作用、适用场景和不适用场景。
- 用 Mermaid 图展示自动流程和状态机。
- 记录 workspace 目录结构、runtime 文档包、状态文件和事件流。
- 分组列出命令，并说明自动流程、手动门禁、恢复和 finish。
- 说明安装、Codex CLI 环境变量、代理、CodeGraph、旧 workspace upgrade。
- 记录 connector contract，包括 issue update、issue agent、reviewer JSON 和 PR connector。
- 保留框架仓库治理和验证命令。
- 同步 CLI help，让 README 中记录的已实现状态与校验命令能在 `sandrone --help` 中被发现。

## 非目标

- 不修改 CLI 行为。
- 不修改默认 connector、prompt 或 reviewer schema。
- 不新增前端 UI。
- 不创建定时任务或 LaunchAgent 文件。
- 不改变 `list`、`status` 或 `validate` 的行为，只补齐 help 文本。

## 验收标准

- README 是完整中文手册，包含项目作用、流程可视化、命令、使用方式、注意事项、配置和环境。
- README 中的命令与当前 CLI 实现保持一致，包含已实现的 `list`、`status`、`validate` 等状态入口。
- `sandrone --help` 列出 `list`、`status [REQ-0001]` 和 `validate`。
- proposal 索引通过校验。
- Markdown 不含尾随空白。
