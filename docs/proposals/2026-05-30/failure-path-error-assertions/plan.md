# 计划: Failure Path Error Assertions

## 目标依赖图

1. 测试断言收紧。
   先替换通用失败断言，避免后续测试继续弱化错误匹配。
2. review gate 失败路径校验。
   依赖测试断言收紧，确保 reviewer rejection 和 approval rejection 都匹配具体错误。
3. 项目治理补充。
   依赖测试语义确定，把要求写入 constitution 和 proposal artifacts。

## 代码改动

- 修改 `tests/cli_flow.rs`:
  - 将 `assert_failure(output)` 收紧为 `assert_failure_contains(output, expected_stderr)`。
  - 在 helper 中同时断言命令失败和 stderr 包含预期文本。
  - 为 plan 前 git pull、plan approval、change-doc approval、reviewer rejection 和 stale approval 失败路径补充错误文本。
- 修改 `.specify/memory/constitution.md`:
  - 在 PR gate 中要求 `cargo test` 必须通过。
  - 要求失败路径测试必须匹配预期错误文本，review gate 必须匹配具体 reviewer 或 approval 错误。
- 新增本 proposal artifacts，并更新 `proposal.json`。

## 测试策略

- 运行 targeted review gate 测试，确认 review 拒绝会匹配 `PlanReviewer rejected` 和 `DesignReviewer rejected`。
- 运行全量 `cargo test`，确认所有测试成功。
- 运行 proposal 校验，确认本次治理变更可被索引发现。

## 风险与回滚

- 风险: 错误文本变更会导致测试失败。这是预期约束，表示用户可见错误发生了变化，测试应随行为一起更新。
- 回滚: 可以恢复 helper 和调用点，但不建议，因为会降低 failure path 测试的诊断能力。
