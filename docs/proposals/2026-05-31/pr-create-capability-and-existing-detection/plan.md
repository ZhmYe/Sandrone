# Plan: PR Create Capability And Existing Detection

## 目标与顺序

1. 先补生成内容测试，要求默认 `pr-create.sh` 记录新 contract 和已有 PR 检查。
2. 补 finish 集成测试，让自定义 connector 返回 `existing<TAB>url`，确认 CLI 输出 `PR already exists`。
3. 扩展 `run_pr_tool` 成功输出解析，兼容裸 URL 和 TSV 状态。
4. 更新默认 GitHub `pr-create.sh`，先检查 gh/repo，再检查已有 PR，最后创建。
5. 更新 README、workflow skill、proposal 索引。
6. 运行完整验证。

## 实现位置

- `src/main.rs`: `DeliveryResult`、`run_pr_tool`、默认 `pr-create.sh`。
- `tests/cli_flow.rs`: 默认脚本契约断言和 existing PR finish 测试。
- `README.md`、`skills/sandrone/SKILL.md`: connector contract。
- `docs/proposals/2026-05-31/pr-create-capability-and-existing-detection/`: 本次变更记录。

## 设计说明

PR 平台差异很大，因此 Rust 只解析 connector 的结构化结果，不内置平台判断。默认脚本继续使用 GitHub CLI，但 connector contract 是平台中立的。`created<TAB>url` 和 `existing<TAB>url` 足够让 CLI、未来前端和机器人区别“新建成功”和“已有可复用 PR”。

旧脚本只输出 URL 的行为保留为兼容路径，避免已有 workspace 的自定义脚本立刻失效。

## 测试策略

- 用生成内容测试确保新 workspace 的默认脚本具备幂等检查。
- 用 finish 端到端测试覆盖 `existing<TAB>url`。
- 保留既有 finish 测试，覆盖旧裸 URL 输出兼容性。
