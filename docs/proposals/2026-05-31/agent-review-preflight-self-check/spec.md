# Agent Review Preflight Self Check Spec

## 背景

自动流程中，planning agent 或 implementation agent 如果在明显不满足 reviewer 标准时直接退出，会触发无意义的 PlanReviewer、TestReviewer 或 DesignReviewer 调用，浪费 token，也增加 review 往返次数。

当前 `issue-agent` 名称容易让人误解为旧的单体 agent。实际设计中，`tools/issue-agent.sh` 是可替换 connector，`tools/prompts/issue-agent.md` 是 planning 和 implementation 共用的共享 agent 契约，阶段要求由 `plan-agent.md` 和 `implementation-agent.md` 承担。

## 需求

- 明确 `issue-agent` 的职责，避免被误删。
- planning agent 在退出前必须按 PlanReviewer 的审查标准自检。
- implementation agent 在退出前必须按 TestReviewer 和 DesignReviewer 的审查标准自检。
- 如果自检发现会产生 critical/high 的缺口，agent 必须先修复或 block，不得直接交给 reviewer。
- 自检结果必须写入 `agent-journal.md`；implementation 阶段还要写入 `change-doc.md` 摘要。
- 默认生成的 prompt、Skill 文档和 README 都必须同步说明该规则。

## 非目标

- 不改变 reviewer schema。
- 不改变 tick/advance 状态机。
- 不删除 `issue-agent` connector 或共享 prompt。
- 不引入新的 reviewer 后端。
