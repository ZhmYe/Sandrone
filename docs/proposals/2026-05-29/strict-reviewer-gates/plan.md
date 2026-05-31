# 计划: Strict Reviewer Gates

## 目标依赖图

1. Review 输出模型。
   先定义 reviewer JSON、阻断规则和落盘路径。
2. 默认 reviewer 连接器和 prompts。
   依赖输出模型，保证默认可运行且可替换。
3. CLI review 命令。
   依赖连接器和输出模型，实现 approval 联动。
4. Upgrade 和文档。
   依赖 CLI 语义稳定。

## 代码改动

- 修改 `src/main.rs`:
  - 新增 reviewer 常量、reviewer 定义和结果结构。
  - 初始化 workspace 时写入 reviewer scripts、prompts 和 schema。
  - `upgrade` 补齐缺失 reviewer assets，并保留已有自定义脚本。
  - 新增 `plan-review` 命令。
  - 新增 `code-review` 命令。
  - 新增 review JSON 解析、blocking 判定、summary 写入和 approval 写入 helper。
- 修改 `tests/cli_flow.rs`:
  - 断言新 workspace 包含 reviewer assets。
  - 覆盖 plan-review high 阻断和通过后 approval。
  - 覆盖 code-review 需要两个 reviewer 全部通过。
- 修改 README 和 skill:
  - 说明 reviewer gate、结构化输出、可替换后端和自动 approval 条件。
- 新增 proposal artifacts 并更新 `proposal.json`。

## 测试策略

- 使用可替换 shell 脚本模拟 reviewer 输出 JSON。
- 先让测试因缺少命令和 review 文件失败，再实现 CLI。
- 全量运行格式化、检查、clippy、测试和 proposal 校验。

## 风险与回滚

- JSON 解析为轻量实现，不替代完整 JSON parser；schema 主要约束默认 LLM 输出。
- 默认 reviewer 使用 `codex exec`，如果目标机器无 Codex CLI，会输出 blocking JSON。用户可替换脚本接入其他后端。
- 自动 approval 更严格后，自动化可能更频繁停在 review gate；这是预期的安全边界。
