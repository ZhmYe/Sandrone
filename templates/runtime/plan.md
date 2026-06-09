---
sandrone_schema: 1
request_id: {{request_id}}
document_type: plan
agent_phase: planning
agent_status: draft
agent_ready_for_review: false
format_check_status: not-applicable
format_check_exit_code: ""
updated_at: {{updated_at}}
---

# 计划: {{request_id}} {{title}}

## 规范化需求记录

- Request ID: `{{request_id}}`
- External ID: `{{external_id}}`
- Source: `{{source}}`
- URL: {{url}}
- 需求来源: {{request_link}}

### 需求名称

{{title}}

### 需求描述索引

本计划模板不内嵌需求正文或摘要，以避免长 issue、父需求或 slice 内容把 planning agent 上下文撑爆。planning agent 必须按需读取下列权威来源；不要把这些来源的全文复制进本文件。

- 需求记录: {{request_link}}
- 父需求记录: {{parent_request_path}}
- 需求拆解: {{decomposition_link}}
- Slice 元数据: {{slice_meta_path}}
- DAG: {{dag_path}}

如果这是 slice，本文件同时是 slice request 与 plan；父需求全文只在确实需要时通过父 request.md 或外部 URL 读取。planning agent 必须阅读标题、权威路径和必要的完整外部需求后再填写本计划，不得把标题当作全部需求。

## 模板说明

这是空白计划模板。`sandrone` 只创建文档包和导航，不生成真实开发计划。planning agent 可以重写正文，但必须保留并更新上面的规范化需求记录。

## 图谱导航

- 需求记录: {{request_link}}
- 需求拆解: {{decomposition_link}}
- Agent 日志: {{agent_journal_link}}
- 实现文档: {{change_doc_link}}
- CodeGraph 上下文: `obsidian/codegraph/context.md`

## 需求理解

待填写。说明标题、完整描述、用户约束、非目标和验收边界。

## 计划前检查

{{preflight_notes}}

## 目标与依赖顺序

待填写。列出目标、先后关系、依赖理由和完成信号。

## 仓库分析

待填写。列出已阅读的文件、模块、现有模式、目标项目文档和 CodeGraph 证据。

## Obsidian 导航

待填写。列出父 request、slice、已完成依赖 slice、相关决策、review 或 PR 链接。这里只写关系，不复制长文档。

## 目标项目内部要求

待填写。列出目标项目自己的 change doc、pre-commit、文档检查、format/lint/test、AI review、安全规则、敏感信息规则和禁止 panic/硬编码等要求。

## 实现计划

待填写。列出预计修改或新增的文件、模块、函数、结构体、命令、配置和迁移方式。说明是否包含破坏性改动，如何兼容旧数据。

## 测试与验证

待填写。列出单元测试、集成测试、失败路径测试、回归测试、安全检查、pre-commit、文档检查、AI review 和人工验证步骤。每条验证都要说明命令或证据。

## 风险、迁移与回滚

待填写。说明兼容性、迁移策略、回滚步骤、数据风险、外部依赖和人工事项。

## 审批门禁

plan gate 通过前不得 start。change-doc gate 通过前不得 finish、commit、push、创建 PR 或 merge。
