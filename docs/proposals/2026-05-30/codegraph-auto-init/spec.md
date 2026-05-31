# Spec: CodeGraph Auto Init

## 背景

CodeGraph MCP 查询要求目标仓库已经存在 `.codegraph/` 索引目录。当前框架 prompt 已要求 planning agent 和 reviewer 参考 CodeGraph 文档，但 CLI 只提示需要 CodeGraph，没有在 clone 或计划前自动初始化索引，导致 Codex 调用 CodeGraph 工具时经常看到 “CodeGraph not initialized”。

## 目标

- 非空 `new --url` clone 后自动尝试运行 `codegraph init -i dev/repo`。
- `plan` preflight 在目标仓库有 commit 且 `.codegraph` 缺失时也自动尝试初始化。
- 初始化失败或 CodeGraph CLI 缺失只记录 warning，不让 `new` 或 `plan` panic。
- `doctor` 展示 CodeGraph CLI 和 `dev/repo/.codegraph` 状态。
- 文档和 skill 说明 `.codegraph` 索引与 `docs/codegraph/context.md` 架构文档的区别。

## 非目标

- 不在 CLI 中复刻 `codegraph-project-preview` skill。
- 不自动生成 HTML 架构图。
- 不把 CodeGraph 缺失作为创建 workspace 的阻塞条件。
- 不改变 reviewer gate 或 agent 状态机。

## 行为要求

- 空仓库跳过 CodeGraph 初始化。
- 有内容仓库优先检查 `dev/repo/.codegraph`，存在则不重复初始化。
- 如果 `codegraph init -i dev/repo` 成功但没有生成 `.codegraph`，记录 warning。
- `docs/codegraph/context.md` 缺失或过期时，plan preflight 仍提示运行 `codegraph-project-preview` skill。

## 验证

- 用 fake `codegraph` 命令验证 `new --url` 会调用 `init -i dev/repo`。
- 用 fake `codegraph` 命令验证 `plan` preflight 会为后来变成非空的 repo 初始化索引。
- 更新 doctor 测试，确认报告包含 CodeGraph CLI 和 index。
