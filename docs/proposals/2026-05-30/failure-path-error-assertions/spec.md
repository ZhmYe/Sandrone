# 规格: Failure Path Error Assertions

## 背景

review gate 和 approval gate 都是自动化流程的安全边界。测试如果只断言命令失败，可能掩盖错误原因: 命令可能因为 unrelated panic、脚本路径错误、JSON 解析问题或 git 环境问题失败，但测试仍然通过。用户要求所有测试必须成功，且失败逻辑必须显式匹配预期错误，包括 review 逻辑。

## 用户目标

当测试覆盖失败路径时，必须同时验证命令失败和 stderr 中的明确错误信息。review 相关失败路径必须匹配具体 reviewer 或 approval gate 的错误，例如 `PlanReviewer rejected`、`DesignReviewer rejected`、`plan approval required`、`change-doc approval required` 或 `approval is stale`。

## 功能要求

- 删除只检查非零退出码的通用失败断言。
- 新增并使用 `assert_failure_contains(output, expected_stderr)`。
- 所有失败路径测试都必须给出明确错误文本。
- review gate 测试必须匹配被拒绝的 reviewer 名称。
- start/finish 的审批失败测试必须匹配缺失或过期 approval 的错误文本。
- constitution 的 PR gate 必须记录该测试质量要求。

## 非目标

- 不改变 CLI 的业务行为。
- 不改变 reviewer JSON schema。
- 不引入新的依赖。

## 验收标准

- `tests/cli_flow.rs` 中不存在只调用 `assert_failure(...)` 的断言。
- 所有失败路径断言都包含期望错误文本。
- review gate 的失败测试能区分 plan reviewer 拒绝、design reviewer 拒绝和 approval gate 拒绝。
- 全量测试和 proposal 校验通过。
