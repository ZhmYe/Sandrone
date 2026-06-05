# 需求拆解: {{title}}（{{request_id}}）

需求路径: {{request_link}}

## 本阶段目标

将本请求拆解为可独立计划、实现、评审和回滚的 Slice，形成可调度的 DAG。

## 资料索引

- 需求记录: {{request_link}}
- Agent 日志: {{agent_journal_link}}
- 当前分片清单: [decomposition.json](decomposition.json)
- 依赖关系图: [dag.json](dag.json)
- 最终交付文档: {{pr_doc_link}}

## 拆解约束

- 默认只保留一个或多个 Slice，禁止无意义拆分。
- 每个 Slice 需有稳定 `Slice ID`、`英文短名`、`目标`、`输入`、`输出`、`验收标准`、`测试边界`、`冲突域`、`依赖`、`完成信号`。
- 所有 Slice 之间通过 `decomposition.json` 与 `dag.json` 一致。
- `decomposition.json`、`dag.json` 与本文必须保持同步，不可冲突。
- 变更边界优先按 Slice 隔离，避免互相影响。

## Slice 列表（按文件维护）

请按以下列字段逐条补充：

| Slice ID | 英文短名 | 目标 | 依赖 | 冲突域 | 完成信号 |
| --- | --- | --- | --- | --- | --- |
| S01 | main | 待填写 | 无 | 待填写 | 待填写 |

## DAG 与调度说明

- 串行关系：输出可复用或有状态依赖的 Slice 必须串行。
- 并行关系：无冲突域重叠、无强顺序依赖的 Slice 可并行执行。
- 合并关系：同一层完成后再推进下游 slice。

## 分支与状态策略

- Slice 分支建议：`codex/{{request_id_lower}}-sNN-<short-name>`
- 每个 slice 依次经历：`plan -> plan-review -> implementation -> code-review`
- Slice code-review 通过后进入 `slice-finished`，再调度 DAG 中可执行后续 slice
- 全部 slice 完成后进入 `wait-update-pr`，进入 PR 创建/更新环节
- 如发生 PR 冲突，走 `pr-refresh -> RebaseAgent -> IntegrationReviewer` 流程

## DecompositionReviewer 提交前清单

- [ ] 每个 slice 已有明确目标与验收标准
- [ ] `Slice ID`、`英文短名`、`依赖` 和 `冲突域` 无缺项
- [ ] `decomposition.json` 与本文件的 Slice 列表一致
- [ ] `dag.json` 与本文件的依赖关系一致、无环
- [ ] 串行/并行策略符合实现风险与冲突域边界
- [ ] 切片完成信号可被 tick/状态机判定
- [ ] 已准备按顺序进入 plan 阶段
