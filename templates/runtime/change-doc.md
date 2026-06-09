---
sandrone_schema: 1
request_id: {{request_id}}
document_type: change-doc
agent_phase: implementation
agent_status: draft
agent_ready_for_review: false
format_check_status: pending
format_check_exit_code: ""
updated_at: {{updated_at}}
---

# 变更文档: {{request_id}} {{title}}

这是变更文档模板。Codex 必须在实现完成后、请求审批前填写真实内容，并更新上方 Sandrone frontmatter 中的文档提交状态。本文档的重点是解释需求如何被实现，而不是完整罗列所有文件变更。

## 导航

- 上级索引: 请从当前 slice index 进入本文档，保持 `project -> parent request index -> slice index -> stage documents` 的主链路。
- Relations: `obsidian/relations.md`
- Request / Plan: {{request_wikilink}}
- Approved plan: {{plan_wikilink}}
- Decomposition: {{decomposition_wikilink}}
- Agent journal: {{agent_journal_wikilink}}
- CodeGraph context: `obsidian/codegraph/context.md`
- Review details: [[reviews]]

本节只保留链接和短说明，不复制完整 plan、完整 reviewer JSON 或长篇文件清单。

## 摘要

用几句话说明实际完成了什么、用户可见变化是什么、是否偏离已批准计划/拆解，以及是否存在剩余风险。

## 实现前后对比

- 实现前: 描述原有流程、缺失能力、失败模式或用户痛点。
- 实现后: 描述新流程、新能力、用户如何观察到变化，以及哪些行为保持兼容。

## 关键设计点

按关键点分别说明设计与实现方式。每个关键点应包含: 为什么这样设计、核心数据/命令/流程是什么、如何满足原始需求、边界和取舍是什么。

## 变更范围摘要

用总结性的方式列出主要改动区域，例如 CLI 命令、状态文件、模板、测试、文档或迁移逻辑。只列关键文件或模块，不需要完整文件清单。

## 目标项目内部要求

- 已阅读的目标项目文档: 填写文档路径。
- 目标项目 change doc: 填写路径或 `Not required`，并说明原因。
- Pre-commit: 填写命令和结果，或 `Not required`。
- 文档检查: 填写命令和结果，或 `Not required`。
- Format/lint/test: 填写命令和结果，必须包含 `tools/check-format.sh --format` 与 `tools/check-format.sh --check` 的通过、失败修复或明确 skip 证据。
- AI review: 填写发现、处理状态，或 `Not required`。
- 所有目标项目内部要求是否完成: 填写 yes/no 和阻塞项。

## 文档与 Checklist

- 已更新的文档: 填写路径和摘要；如果没有目标项目文档需要更新，填写 `Not required` 并说明原因。
- 所有交付文档中的 checklist 是否已全部打勾: 填写 yes/no，并列出检查过的文档路径。
- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: 填写 yes/no。
- 已批准 plan 中的历史 checklist 不要为了凑勾而篡改；如果执行结果与 plan checklist 不一致，在本 change-doc 解释。

## 后续流程

记录当前自动流程无法完成但仍需追踪的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。不要把这些事项保留为未勾选 checklist。

## 验证证据

填写准确命令、输出摘要、失败修复过程和人工验证证据。日志、错误、commit hash、测试输出保持原文。

必须记录格式门禁结果: `tools/check-format.sh --check` 通过、失败修复或明确 skip；如曾失败，引用 `status.json` 中的失败 reason，并说明修复方式。

## Review 结果

尚未产生 review 结果。

## 审批门禁

填写完成后等待 wrapper hook 调用外层 `sandrone advance` 提交 change-doc gate 并运行 code-review。审批通过前不得运行 `sandrone finish --request_id {{request_id}}`，也不得 commit、push、创建 PR 或 merge。
