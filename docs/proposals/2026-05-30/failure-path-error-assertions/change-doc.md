# 变更文档: Failure Path Error Assertions

## 摘要

本次变更把失败路径测试从“只确认失败”收紧为“确认失败且匹配预期错误”。review gate 的失败测试会明确匹配被拒绝的 reviewer，approval gate 的失败测试会匹配缺失或过期 approval 的错误。

## 实现前后对比

- 实现前: 测试 helper 只判断命令是否非零退出，无法证明失败原因符合预期。
- 实现后: 所有失败路径测试都必须传入预期 stderr 片段。命令因为其他原因失败时，测试会失败并打印实际 stderr，便于定位。

## 关键设计点

### 失败原因成为测试契约

`assert_failure_contains` 同时检查 `output.status.success() == false` 和 stderr 文本。这样每个失败路径都绑定到用户可见错误，例如 `plan approval required`、`change-doc approval required`、`approval is stale`。

### Review Gate 精确匹配

`plan-review` 的拒绝路径匹配 `PlanReviewer rejected`，`code-review` 的拒绝路径匹配 `DesignReviewer rejected`。这保证测试不是因为任意 review 脚本失败或环境问题而误通过。

### Constitution 固化质量要求

PR gate 增加 `cargo test` 必须通过，并要求 intentional failure path tests 匹配预期错误文本。后续新增失败路径或 review gate 测试时，必须延续这个模式。

## 变更范围摘要

主要改动为测试 helper 与失败路径断言、项目 constitution，以及本次 proposal artifacts。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`

## 风险与后续

- 如果未来调整 CLI 错误文案，需要同步更新对应测试断言。
- 后续可以进一步把错误文本集中成常量，降低文案变更时遗漏测试的概率。
