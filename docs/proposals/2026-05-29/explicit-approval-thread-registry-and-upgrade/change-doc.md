# 变更文档: Explicit Approval, Thread Registry And Upgrade

## 摘要

本次变更把原先依赖对话自觉停顿的流程，升级为 CLI 强制检查的显式 approval 门禁；同时加入 session registry，让后续机器人或前端可以看到每个 request 的 planning / implementation thread；并提供 `upgrade`，让已经创建过的 workspace 可以迁移到新结构。runtime 模板、handoff、README 和 skill 也改为中文优先。

## 实现前后对比

- 实现前: `plan` 和 `change-doc` 审批只是提示词约束，Codex 在当前 thread 中继续执行也不会被 CLI 拦住。`start` 不检查计划是否审批，`finish` 不检查变更文档是否审批。旧 workspace 只能保留创建时的模板和 skill，没有同步新框架能力的命令。
- 实现后: `submit/approve/reject/approvals` 会写入可审计 approval 文件，`start` 强制检查 plan approval，`finish` 强制检查 change-doc approval，并通过 `artifact_sha256` 检测审批后文档被修改的 stale 状态。`session/sessions` 登记可见 thread 信息，`upgrade` 可以补齐旧 workspace 的 schema、session registry、approval 目录、中文模板和最新 skill。

## 关键设计点

### 显式 Approval

Approval 存在于 `docs/changes/<name>/approvals/<gate>.approval.json`。`submit` 记录当前 artifact 和 hash，`approve/reject` 写入审批决定。`start` 和 `finish` 不再相信对话中的“已经批准”，而是读取 approval 文件并重新计算 artifact hash。这样机器人、前端和 CLI 都能共享同一份审批事实。

### Artifact Stale 检查

审批文件记录 `artifact_sha256`。如果 `plan.md` 或 `change-doc.md` 审批后被修改，后续 `start` 或 `finish` 会失败并提示 approval stale。这个设计避免“审批的是 A，执行的是 B”的问题。

### Session Registry

`.sandrone/sessions.json` 记录 request、phase、status、thread_id、thread_url、workspace、target_repo、worktree 和 change_path。CLI 不负责创建 Codex App thread，但它负责提供可见登记点，后续机器人创建 thread 后可以写回 URL，前端也可以读取展示。

### Upgrade 旧 Workspace

`upgrade --dry-run` 先展示迁移动作，`upgrade` 再执行。迁移会补齐 schema version、session registry、approval 目录、中文模板和最新 skill。它保留 `dev/repo`、`dev/worktrees`、用户改过的 issue connector，以及已经填写过的计划/变更文档。

### Change Doc 写作标准

生成的 `change-doc.md` 不再要求完整文件列表，而是要求说明需求如何被实现。模板包含摘要、实现前后对比、关键设计点、变更范围摘要、目标项目内部要求、验证证据和审批门禁。这样审批人能看到设计和行为变化，而不是被文件流水账淹没。

## 变更范围摘要

主要改动集中在 CLI 状态流、approval JSON、session registry、upgrade 迁移逻辑、runtime 中文模板、skill 指南、README 和集成测试。测试覆盖了审批缺失、审批过期、change-doc 门禁、session 登记、旧 workspace 升级和中文模板内容。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`

## 风险与后续

- JSON 解析仍是轻量实现，后续接机器人时可以考虑引入 `serde_json`。
- 目前 CLI 只登记 thread，不负责创建 Codex App thread。真正自动创建可见会话需要 Codex 平台或机器人层能力。
- `upgrade` 会保守地保留用户内容；如果旧模板已经被用户部分填写但仍带有默认模板标记，用户应先备份或手动调整后再升级。
