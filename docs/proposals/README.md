# Proposal 归档说明

这里保存 Sandrone 框架自身的历史 proposal、plan、task 和 change-doc。它们用于追溯项目演进，不是当前 runtime workspace 的操作规范。

## 当前规范入口

当前实现和使用方式请优先阅读：

- [../README.md](../README.md)
- [../workflow.md](../workflow.md)
- [../workspace-layout.md](../workspace-layout.md)
- [../commands.md](../commands.md)
- [../operations.md](../operations.md)
- [../development.md](../development.md)
- `templates/prompts/*.md`
- `skills/sandrone/SKILL.md`

## 关于旧术语

早期 proposal 中可能出现以下旧设计术语：

- `approval JSON`
- `approvals/`
- `status.json.gates`
- `artifact_sha256`
- `docs/changes`
- `checks/format-check.md`
- `.sandrone/state/agents/*.success`

这些内容只代表当时的设计记录。当前模型已经迁移为：

- runtime 文档位于 `obsidian/changes/`。
- 父 request 负责需求、拆解、PR 汇总；slice 负责 plan、implementation 和 change-doc。
- 阶段 Markdown frontmatter 是文档提交状态、format/check 摘要和 `gate_*` 门禁状态的事实源。
- `status.json` 只保存 request/slice runtime 阶段、阻塞原因、worktree/branch/PR 等机器状态。
- `sdr upgrade` 会迁移旧 workspace 的旧 approval/gate/success/format-check 记录。

维护新功能时，不能直接复制旧 proposal 中的旧状态方案；应基于当前文档和测试更新。
