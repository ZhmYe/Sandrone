# Plan: Agent Baseline Failure Repair

## 实施步骤

1. 增加生成内容测试，要求 implementation prompt 和 TestReviewer prompt 包含 Baseline failure 规则。
2. 更新 implementation agent prompt，要求修复非本分支改动导致的已有测试失败。
3. 更新 TestReviewer prompt，要求审查 Baseline failure 是否被修复和记录。
4. 更新 README、workflow skill、proposal 索引。
5. 运行格式、编译、clippy、测试、proposal 校验和 diff 检查。

## 改动位置

- `src/main.rs`: 默认 implementation agent prompt 和 TestReviewer prompt。
- `tests/cli_flow.rs`: 默认 workspace 生成内容断言。
- `README.md`: agent 测试失败处理规则。
- `skills/sandrone/SKILL.md`: skill 中的 agent/reviewer 规则。
- `docs/proposals/2026-05-31/agent-baseline-failure-repair/`: 本次框架变更记录。

## 风险与兼容

- 该规则扩大了 agent 的修复责任，但仍限定在当前 worktree 内。
- 如果修复需要外部权限、数据或会破坏 approved plan，agent 必须 block，不得擅自扩大范围。
