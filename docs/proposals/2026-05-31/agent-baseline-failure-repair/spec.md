# Spec: Agent Baseline Failure Repair

## 背景

implementation agent 在运行测试时，可能发现失败并不是本 request 分支直接引入的，而是目标项目已有测试、fixture、依赖或配置问题。此前 prompt 容易让 agent 把这类失败当作“非本分支问题”记录后忽略，导致最终交付仍然无法通过项目验证。

## 目标

- implementation agent 必须修复测试过程中发现的已有失败，即使失败不是由本分支改动直接导致。
- TestReviewer 必须审查这类 Baseline failure 是否被修复并记录。
- change-doc 必须记录 Baseline failure 的失败命令、根因、修复范围和复验结果。
- 只有修复会破坏 approved plan、需要外部权限/数据、或无法安全判断时，agent 才能 block。

## 非目标

- 不允许 agent 随意重构无关代码。
- 不允许为了修复 baseline failure 删除、跳过或弱化测试。
- 不允许绕过 approved plan、reviewer gate 或目标项目要求。

## 行为要求

- 测试失败后，agent 必须判断是否可以在当前 worktree 安全修复。
- 可以安全修复时，必须修复并复验。
- 不能安全修复时，必须 block，并写清恢复步骤。
- `agent-journal.md` 和 `change-doc.md` 必须包含 Baseline failure 记录。
- TestReviewer 发现 agent 把“不是本分支改的”作为忽略理由时，必须给 high；如果关键测试无法通过且没有安全 block，必须给 critical。

## 验证

- 新 workspace 生成的 implementation prompt 包含“不是由本分支改动导致的已有测试失败”和 Baseline failure。
- 新 workspace 生成的 TestReviewer prompt 包含同样的审查规则。
