# PR 交付文档: {{request_id}} {{title}}

这是该需求的「Finish 阶段总文档」。本文件作为 PR 交付与合并状态的汇总入口，避免把实现细节重复拆散在多个文件里。

## 工作流导航

- 上级索引: 请从父 request index 进入本文档，保持 `project -> parent request index -> stage documents` 的主链路。
- Project Relations: `obsidian/relations.md`
- Request: {{request_wikilink}}
- Plan: {{plan_wikilink}}
- Change Doc: {{change_doc_wikilink}}
- 状态文件: [[status|status.json]]
- PR/合并状态快照: [[status.json|status.json]]

## PR 文档清单

- PR 标题: {{pr_title}}
- 分支: `{{branch}}`
- 关联请求: `{{request_id}}`
- 当前阶段状态: `{{status}}`

## 交付摘要

待填写。请写明已完成了哪些交付动作、PR 是否创建/更新、是否有冲突处理、是否等待合并。

## PR 操作记录

- 交付时间: `{{delivered_at}}`
- 本地 PR 工具输出: `{{pr_tool_output}}`
- 关联 PR URL: `{{pr_url}}`（无则保留 `n/a`）
- PR 状态脚本结果: `{{pr_status_raw}}`

## 审批与评审信息

- PR 创建/更新是否成功: `{{pr_status}}`
- Change Doc 审核是否通过: `{{change_doc_approved}}`
- PR 状态门禁: `{{pr_status_gate}}`

## 风险与后续

如 PR 冲突、base/master drift、持续集成异常或人工审阅阻塞，请在此保留处理路径，不要写到未勾选 checklist。`pr-status=unsafe` 会退回 implementation/code-review；待后续流程完结后再将状态迁移到 `finished`。
