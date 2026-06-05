---
title: "{{project_name}}"
type: project
repo_name: "{{repo_name}}"
git_url: "{{git_url}}"
base_branch: "{{base_branch}}"
updated: {{updated_at}}
tags:
  - Sandrone
  - Sandrone/project
---

# {{project_name}}

> 这是 Sandrone 自动维护的 Obsidian 项目根节点。它只直接链接父 request index，避免 project 图谱直接连到 slice 或阶段文档。

## 项目信息

| 字段 | 值 |
| --- | --- |
| Repo | `{{repo_name}}` |
| Git URL | `{{git_url}}` |
| Base branch | `{{base_branch}}` |
| Updated | `{{updated_at}}` |

## 状态汇总

{{status_summary}}

## 需求索引

{{request_index}}

## 派生文件

- `obsidian/relations.md`
- `obsidian/views/requests.base`
- `obsidian/views/slices.base`
- `obsidian/project.canvas`
- `obsidian/derived/requests.json`
- `obsidian/derived/slices.json`

> `derived/*.json` 是 AI 优先读取的轻量索引；`.base` 和 `.canvas` 是从 request/status/DAG 派生的人类视图，不要手写维护。为保持图谱主链路清晰，本 project note 不直接链接这些派生文件。

## 固定导航

- `obsidian/changes/`
- `obsidian/codegraph/context.md`
