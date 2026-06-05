# Obsidian 变更图谱

每个 workspace 都是一个独立 Obsidian vault。`.obsidian/` 只放 Obsidian 配置；正文笔记在 `obsidian/`。

## 设计目标

- 人类能快速看到项目、需求、slice、PR 和阻塞状态。
- AI 能用少量文件恢复上下文，不必每次扫描全部历史和全部代码。
- 权威状态仍由 `status.json`、`requests.tsv`、`decomposition.json`、`dag.json` 和 review detail 承担。
- Canvas、Base、derived JSON 都可以重建，不是人工维护的事实源。

## 主链路

固定图谱链路：

```text
project.md -> 父 request index -> slice index -> 阶段总文档
```

规则：

- `project.md` 只直接链接父 request index。
- 父 request index 链接 request、decomposition、pr-doc、agent journal、slice index。
- slice index 链接 plan、change-doc、review detail、slice agent journal。
- journal 不承担主导航，不需要反向链接所有阶段文档。
- 阶段文档不要反向链接 `project.md`，避免图谱变成一团。
- 旧的 `docs/changes` 不再使用；runtime 变更文档统一在 `obsidian/changes/`。

## 目录

```text
obsidian/
  project.md
  relations.md
  codegraph/context.md
  derived/
    requests.json
    slices.json
  views/
    requests.base
    slices.base
  project.canvas
  changes/
    2026-06-05-req-0001-short-name/
      REQ-0001 index.md
      REQ-0001 request.md
      REQ-0001 decomposition.md
      REQ-0001 pr-doc.md
      REQ-0001 agent-journal.md
      decomposition.json
      dag.json
      status.json
      slices/
        S01/
          REQ-0001-S01 index.md
          REQ-0001-S01 plan.md
          REQ-0001-S01 change-doc.md
          REQ-0001-S01 agent-journal.md
```

## 文件职责

| 文件 | 职责 |
| --- | --- |
| `project.md` | vault 根导航，按日期索引父 request。 |
| `relations.md` | 轻量关系入口，当前不参与调度算法。 |
| `derived/requests.json` | request 轻量索引，适合 agent 先读。 |
| `derived/slices.json` | slice 轻量索引，包含父 request、依赖、状态、branch、worktree。 |
| `views/*.base` | Obsidian Bases 派生视图。 |
| `project.canvas` | 从 project/request/slice 派生的 JSON Canvas，用于观察。 |
| `<REQ> index.md` | 父 request 导航入口和状态摘要。 |
| `<REQ> request.md` | 固化需求来源、标题、描述和 URL。 |
| `<REQ> decomposition.md` | request 拆解说明、slice 列表、DAG 解释、小型需求覆盖表。 |
| `<REQ> pr-doc.md` | PR/finish 阶段汇总入口。 |
| `<REQ> agent-journal.md` | 父 request agent 的过程日志。 |
| `<REQ-SNN> index.md` | slice 导航入口。 |
| `<REQ-SNN> plan.md` | slice request 与计划。 |
| `<REQ-SNN> change-doc.md` | slice 实现说明、验证和 review 处理摘要。 |
| `<REQ-SNN> agent-journal.md` | slice agent 的过程日志。 |

## 派生文件

随时可以运行：

```bash
sdr obsidian-refresh
```

它会刷新：

- `obsidian/project.md`
- `obsidian/relations.md`
- `obsidian/derived/requests.json`
- `obsidian/derived/slices.json`
- `obsidian/views/*.base`
- `obsidian/project.canvas`

agent 应优先读 `derived/*.json`、`dag.json`、`decomposition.json` 和当前 request/slice 的 index，再进入具体阶段文档。Canvas 适合人类观察，AI 不应把它当唯一事实源。

## 内容风格

好的 Obsidian change trace 应该回答：

- 这个 request 是什么？
- 拆成了哪些 slice，为什么这样拆？
- 哪个 slice 下一步可运行？
- 哪些 gate 已通过，证据在哪里？
- 哪个 branch/worktree 包含工作？
- PR 是否创建、更新、冲突、合并？
- 如果 blocked，如何恢复？

避免：

- 在 index 里复制完整 plan、完整 change-doc、完整 reviewer JSON。
- 写过大的需求映射矩阵。需求覆盖只需要小表格：原始验收点、覆盖 slice、验证方向。
- 手写 generated Canvas/Base/derived JSON 表达状态。
- 让 `project.md` 直接连到每个 slice 或阶段文件。
