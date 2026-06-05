# Agent Review Preflight Self Check Plan

## 目标与依赖顺序

1. 先在集成测试中增加 prompt 内容断言，确保默认 workspace 生成的 prompt 明确包含 reviewer 提交前自检要求。
2. 更新默认 `issue-agent.md` 共享契约，说明它不是过时文件，并添加 planning/implementation 通用自检规则。
3. 更新默认 `plan-agent.md`，把原有自检清单升级为 `PlanReviewer 提交前自检清单`，要求逐项核对 reviewer 标准。
4. 更新默认 `implementation-agent.md`，增加 `Code Review 提交前自检`，分别覆盖 TestReviewer 和 DesignReviewer 的严格检查项。
5. 更新 source skill 和 README，让人类和后续 Codex 都能理解 `issue-agent` 的职责与自检门槛。
6. 运行现有 Rust、clippy 和 proposal 校验。

## 代码改动位置

- `src/main.rs`: 修改默认 prompt 生成函数。
- `tests/cli_flow.rs`: 增加默认 prompt 内容断言。
- `README.md`: 补充 `issue-agent` 解释和提交前自检说明。
- `skills/sandrone/SKILL.md`: 补充 agent 要求。
- `proposal.json`: 登记本次 proposal。

## 风险

- 这是 prompt 和文档层面的收紧，不改变状态机，兼容现有 workspace。
- 旧 workspace 只有在运行 `sandrone upgrade --default` 或手动复制 `.example` 后才会替换正式 prompt。
- 如果用户自定义了 prompt，普通 `upgrade` 只会更新 `.example`，不会覆盖用户文件。

## 验证

- `cargo test new_name_creates_framework_and_empty_target_repo_only`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
