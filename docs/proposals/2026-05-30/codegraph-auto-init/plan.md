# Plan: CodeGraph Auto Init

## 目标与顺序

1. 先写测试，用 fake `codegraph` 命令捕获 CLI 调用。
2. 新增 CodeGraph 初始化结果类型，区分 skipped、already initialized、initialized、unavailable、failed。
3. 在 `new --url` 的非空仓库分支调用初始化。
4. 在 `assess_repository_before_planning` 中补一次初始化。
5. 更新 `doctor`、README、workflow skill 和默认 prompt。
6. 运行完整验证。

## 实现位置

- `src/main.rs`: CodeGraph helper、clone 流程、plan preflight、doctor、默认 prompt。
- `tests/cli_flow.rs`: fake CodeGraph 测试与 doctor 断言。
- `README.md`、`skills/sandrone/SKILL.md`: 生命周期说明。

## 兼容性

这是向后兼容变更。CodeGraph CLI 不存在或失败时，流程只输出 warning，用户仍可继续创建 workspace 或计划，但 plan preflight 会保留风险提示。

## 测试策略

- fake `codegraph` 创建 `.codegraph` 目录并记录参数，避免测试依赖真实本机索引。
- 继续保留空仓库跳过 CodeGraph 的测试。
- 全量运行 Rust 测试和 proposal 校验。
