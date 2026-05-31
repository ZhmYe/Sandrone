# Plan: Comprehensive Chinese README

## 实现计划

1. 重写 README 顶部定位，说明项目是目标仓库外层自动开发框架，而不是目标仓库替代品。
2. 增加核心原则、目录结构和路径职责表。
3. 用 Mermaid `flowchart` 和 `stateDiagram` 展示自动流程和 request 生命周期。
4. 分章节整理安装、环境变量、Codex CLI、代理、CodeGraph 和快速开始。
5. 按 Workspace、Request、自动推进、手动门禁、阻塞恢复和交付分组列命令。
6. 记录 runtime 文档包和 connector contract。
7. 补充自动化运行、finish/PR、旧 workspace upgrade、排障恢复、安全质量要求和框架治理。
8. 增加本 proposal 文档包并更新 `proposal.json`。
9. 为 CLI help 增加 `list`、`status [REQ-0001]` 和 `validate`，并补一个回归测试保证 help 与 README 不再分叉。

## 影响范围

- `README.md`
- `proposal.json`
- `src/main.rs`
- `tests/cli_flow.rs`
- `docs/proposals/2026-05-31/comprehensive-chinese-readme/`

## 风险与控制

- 风险: README 过长导致第一次阅读成本上升。
  控制: 采用清晰章节、表格和代码块，先给项目作用和快速开始，再给深入契约。
- 风险: 文档描述与 CLI 行为不一致。
  控制: 对照 `codex-auto-dev --help` 和源码中的 `list`、`status`、`validate` 实现后再写命令表。
- 风险: Mermaid 语法在 GitHub 渲染失败。
  控制: 使用简单 `flowchart TD` 和 `stateDiagram-v2`，避免复杂样式。

## 验证

- `codex-auto-dev --help`
- `cargo test help_lists_state_and_validation_commands`
- `git diff --check`
- `python3 scripts/validate_proposals.py`
