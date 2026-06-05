# Agent Journal: {{request_id}} {{title}}

这个文件用于避免上下文过长后无法恢复。agent 每轮都必须追加记录: 当前阶段、读取的文件、review 发现、修改内容、运行命令、剩余风险和下一步。

## 说明

`{{agent_journal_file}}` 仅用于文本化记录执行过程，不再承载导航链接。真实链路由 `{{request_id}} index.md -> {{agent_journal_file}} + 阶段总文档` 维护（本文件是上下文恢复入口）。

## 记录模板

```markdown
## Attempt <n> - <decomposition|planning|implementation|rebase>

- Read:
- Changed:
- Reviewer findings:
- Validation:
- Next:
```
