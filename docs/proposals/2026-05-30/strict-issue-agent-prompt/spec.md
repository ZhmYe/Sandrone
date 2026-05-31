# 规格: Strict Issue Agent Prompt

## 背景

issue-agent 是自动流程中真正写计划、写代码和修复 review 的执行者。当前提示词已经说明了门禁和阶段，但对 plan 质量、change-doc 质量、测试证据、journal 记录和 reviewer finding 处理不够具体，容易导致 reviewer 多轮拒绝，降低自动化效率。

## 用户目标

把 issue-agent prompt 扩展成具体作业手册，让 agent 在提交 reviewer 前先按 reviewer 标准自检，减少低质量 plan、缺测试、change-doc 空泛和 review finding 处理不完整的问题。

## 功能要求

- issue-agent prompt 必须包含启动前检查。
- 必须明确 plan 的交付标准和提交 plan-review 前的自检清单。
- 必须明确 implementation 的代码质量、错误处理、敏感信息、可重复运行和 worktree 边界。
- 必须明确测试与验证要求，包括失败路径断言明确错误文本。
- 必须明确 change-doc 交付标准，强调实现方式、前后对比、关键设计点和验证证据。
- 必须明确 code-review 修复循环，要求同时读取 TestReviewer 和 DesignReviewer details。
- 必须明确 journal 格式，每条 critical/high 都要有处理记录。
- 必须明确 block 条件，尤其是 gate unavailable、最大轮数、关键输入不可读和必需验证无法运行。

## 非目标

- 不让 CLI 自己生成真实计划或代码。
- 不放宽 reviewer gate。
- 不让 issue-agent commit、push、PR 或 merge。

## 验收标准

- 新 workspace 的 `tools/prompts/issue-agent.md` 包含启动前检查、Planning 交付标准、Plan 自检、Implementation 交付标准、测试与验证、Change Doc 交付标准和 Block 规则。
- README 和 skill 同步说明 issue-agent prompt 的自检要求。
- 现有自动流程测试继续通过。
