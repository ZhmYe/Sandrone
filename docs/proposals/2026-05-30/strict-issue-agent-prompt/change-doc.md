# 变更文档: Strict Issue Agent Prompt

## 摘要

本次变更把默认 issue-agent prompt 扩展为具体作业手册，要求 agent 在提交 plan-review 和 code-review 前完成自检、补齐文档、运行验证并记录每条 reviewer finding 的处理结果。

## 实现前后对比

- 实现前: issue-agent prompt 主要描述阶段命令和门禁，缺少 plan/change-doc 的具体质量标准，容易被 reviewer 反复拒绝。
- 实现后: prompt 明确启动前检查、plan 交付标准、Plan 自检、implementation 质量要求、测试验证、change-doc 标准、review 修复循环、journal 格式和 block 条件。

## 关键设计点

### 提交前自检

Planning 阶段提交 review 前，agent 必须检查需求理解、目标依赖、仓库分析、目标项目内部要求、实现计划、测试验证、风险回滚和审批门禁。Implementation 阶段提交 code-review 前，agent 必须补齐测试、验证证据和 change-doc。

### Journal 可恢复

每轮都要向 `agent-journal.md` 记录读取内容、修改内容、review finding 处理、验证结果和下一步。每条 critical/high 都必须有处理说明，方便超过轮数后人工恢复。

### Block 比绕过更安全

prompt 明确列出 gate unavailable、关键输入不可读、最大轮数、需求冲突和必需验证无法运行时必须 block，不能修改 reviewer、手动 approval 或跳过门禁。

## 变更范围摘要

主要改动为 `default_issue_agent_prompt`、README、skill、默认 asset 测试和本次 proposal artifacts。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`

## 风险与后续

- issue-agent prompt 更长，会增加上下文占用，但能提升计划和实现的一次通过率。
- 后续可以按 backend 能力拆成短 prompt 和详细 checklist 文件。
