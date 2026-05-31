# 计划: {{title}}

## 规范化需求记录

- Request ID: `{{request_id}}`
- External ID: `{{external_id}}`
- Source: `{{source}}`
- URL: {{url}}

### 需求名称

{{title}}

### 需求描述

{{body}}

## 模板说明

这是计划模板。Codex 或 planning agent 必须填写真实计划；`codex-auto-dev` 只创建必要文档包，不生成实际开发计划。agent 可以重写本文件，但必须保留并更新上面的规范化需求记录。

## 需求理解

读取 `request.md` 的需求标题和需求描述，补齐原始需求中的缺失上下文。标题和描述都必须作为需求来源，标题不能替代完整需求描述。

## 计划前检查

{{preflight_notes}}

## 目标与依赖顺序

列出要完成的目标、目标之间的依赖关系、必须先完成的前置条件，以及每个目标的完成信号。

## 仓库分析

列出已经阅读的文件、模块、现有模式、目标项目文档和 CodeGraph 文档。说明本次改动为什么应该落在这些位置。

## 目标项目内部要求

列出目标项目自己的 change doc、pre-commit、文档检查、format/lint/test 命令、AI review、安全规则、敏感信息规则和禁止 panic/硬编码等要求。

## 实现计划

列出预计修改或新增的文件、模块、函数、结构体、命令、配置和迁移方式。说明是否包含破坏性改动，如何兼容旧数据。

## 测试与验证

列出单元测试、集成测试、失败路径测试、回归测试、安全检查、pre-commit、文档检查、AI review 和人工验证步骤。每条验证都要说明命令或证据。

## 执行任务清单

- [ ] 阅读 `request.md` 和目标项目文档。
- [ ] 填写本计划，覆盖目标、依赖、实现位置、测试策略和风险。
- [ ] 等待 wrapper hook 调用外层 `codex-auto-dev advance` 提交 plan gate 并运行 PlanReviewer。
- [ ] PlanReviewer 拒绝时，读取 `reviews/plan-review/summary.json` 和最新 detail，修复计划后再次交给外层 advance/tick。
- [ ] 计划审批通过后，外层 advance/tick 会创建独立 worktree 并派发 implementation agent。
- [ ] implementation 只能在生成的 worktree 中实现，不直接编辑 `dev/repo`。
- [ ] 填写 `change-doc.md` 后等待 wrapper hook 调用外层 advance 提交 change-doc gate 并运行 TestReviewer 和 DesignReviewer。

## 审批门禁

plan approval 通过前不得 start。change-doc approval 通过前不得 finish、commit、push、创建 PR 或 merge。
