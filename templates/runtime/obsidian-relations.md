---
title: "Project Relations"
type: relations
updated: {{updated_at}}
tags:
  - Sandrone
  - Sandrone/relations
---

# Project Relations

> 这个文件维护跨需求和跨 slice 的轻量关系。它是 AI 省 token 的入口之一：先读这里判断哪些历史需要展开，不要从头阅读全部 change 包。

## 读取协议

1. 先读 `obsidian/project.md` 和本文件。
2. 只展开与当前 request/slice 有关系的 `<REQ> index.md`、`<REQ> agent-journal.md` 和阶段文档。
3. 如果关系缺失，先在下表追加候选关系和原因，再决定是否读取更多历史。

## 关系表

| From | Relation | To | Reason | Confidence | Updated |
| --- | --- | --- | --- | --- | --- |
| 待维护 | related-to | 待维护 | 待维护 | low | {{updated_at}} |

## 关系类型

- `depends-on`: From 需要 To 先完成。
- `blocks`: From 阻塞 To。
- `related-to`: 语义相关，但没有明确依赖。
- `touches-same-area`: 可能修改同一模块或规则域。
- `supersedes`: From 替代 To 的设计或实现。
- `follows-up`: From 是 To 的后续需求。
