# Change Doc: Comprehensive Chinese README

## 摘要

本次将 README 从阶段性说明重写为完整中文用户手册，覆盖项目作用、整体流程、可视化图、命令、使用方式、注意事项、配置、环境、恢复和框架治理。

同时补齐 `codex-auto-dev --help`，让已实现的 `list`、`status [REQ-0001]` 和 `validate` 与 README 命令表保持一致。

## 实现前后对比

- 实现前: README 已有中文内容，但更偏流程摘要和规则清单，缺少完整手册结构；项目作用、目录结构、自动状态机、connector contract、环境配置和排障恢复分散在不同段落。
- 实现后: README 按用户阅读路径组织，先说明项目定位和原则，再给架构、可视化流程、安装环境、快速开始、命令表、connector contract、自动化、finish、升级、排障和治理。

## 关键设计点

### 手册化结构

README 采用“先理解，再运行，再深入”的顺序。前半部分面向首次使用者，解释这个项目解决什么问题、如何初始化 workspace、如何运行 `tick`。后半部分面向维护者和排障场景，记录 connector contract、review JSON、升级、恢复和治理。

### 可视化流程

新增两个 Mermaid 图:

- `flowchart TD`: 展示 update、planning agent、PlanReviewer、worktree、implementation agent、code-review、waiting-finish 和 finish 的推进关系。
- `stateDiagram-v2`: 展示 request 从 discovered 到 finished 或 blocked 的状态路径。

### 命令分组

命令不再堆在一个代码块里，而是按 workspace、request 状态、自动推进、手动门禁、阻塞恢复和交付分组。这样用户可以根据当前任务快速找到入口。

### 环境与排障

README 明确记录 Codex CLI 解析顺序、`~/.zshrc` 配置、GUI/LaunchAgent 场景、代理继承、CodeGraph 配置，以及常见 blocked 原因和恢复命令。

## 变更范围摘要

- 重写 `README.md`。
- 更新 `src/main.rs` 的 help 文本。
- 在 `tests/cli_flow.rs` 中增加 help 回归测试。
- 更新 `proposal.json`，新增 `comprehensive-chinese-readme`。
- 新增本 proposal 的 `spec.md`、`plan.md`、`tasks.md`、`plan.html` 和 `change-doc.md`。

## 验证证据

- [x] `codex-auto-dev --help`
- [x] `cargo test help_lists_state_and_validation_commands`
- [x] `git diff --check`
- [x] `python3 scripts/validate_proposals.py`

## 自动评审意见

本次主要是文档变更，附带 CLI help 文本同步，没有运行模型 reviewer gate。已用 CLI help、回归测试和源码行为核对命令表，并用 proposal 校验脚本确认索引完整。
