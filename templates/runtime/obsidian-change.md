---
title: "{{request_id}} {{title}}"
type: {{kind}}
request_id: {{request_id}}
status: {{status}}
source: {{source}}
external_id: "{{external_id}}"
branch: "{{branch}}"
worktree: "{{worktree}}"
updated: {{updated_at}}
tags:
  - Sandrone
  - Sandrone/{{kind}}
---

# {{request_id}} {{title}} 工作流索引

> 这是 Sandrone 自动维护的 Obsidian 导航笔记。本 request 的可读文档包保存在当前 Obsidian change 目录；`.sandrone/` 仍负责机器索引、事件流、锁和全局 registry。

## 基本信息

| 字段 | 值 |
| --- | --- |
| Request ID | `{{request_id}}` |
| 类型 | `{{kind}}` |
| 状态 | `{{status}}` |
| 来源 | `{{source}}` |
| External ID | `{{external_id}}` |
| URL | {{url}} |
| Branch | `{{branch}}` |
| Worktree | `{{worktree}}` |

## 工作流导航

- 上级导航: {{upstream_index_link}}
- 关系: [[relations|relations.md]]
- Agent 日志: {{agent_journal_link}}
- 阶段总文档:
{{stage_document_links}}
- Slice 索引:
{{slice_index_links}}
- 状态文件: [[status|status.json]]

每个阶段总文档应只保留导航与关键结论，不重复复制完整实现内容或 review JSON；核心语义链路为：
`project -> parent request index -> slice index -> stage documents`。Agent Journal 不反向连接其他阶段文档，避免图谱过密。

## 关系图

```mermaid
flowchart LR
  IDX["{{request_id}} Index"] --> J["Agent Journal"]
{{workflow_mermaid_edges}}
```

## 需求关系

- 父级 Request: 待关联
- 依赖需求: []
- 被依赖需求: []
- 相关 PR: 待关联
- 相关决策: 待关联

## 当前摘要

请由 agent 在计划、实现、PR 刷新或恢复时维护本节的短摘要。这里不复制完整 plan 或 change-doc，只记录足以恢复上下文的导航、关系和关键结论。

## 下一步

根据 `status.json` 和 review gate 决定下一步。常见入口:

- 继续自动推进: `sandrone tick --request_id {{request_id}}`
- 查看 gate 状态: `sandrone gates --request_id {{request_id}}`
- 查看状态: `sandrone status {{request_id}}`
