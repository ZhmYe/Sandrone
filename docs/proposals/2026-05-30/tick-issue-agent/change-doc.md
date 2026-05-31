# 变更文档: Tick Issue Agent

## 摘要

本次变更计划把自动流程收敛为 `tick + issue-agent`。heartbeat 主 session 只负责 update、刷新已结束 agent 状态，并为所有 eligible request 异步启动 issue-agent 子运行；真正的 plan、implementation 和 review 修复循环由每个连续 Codex CLI 子 session 完成。

## 实现前后对比

- 实现前: 手动流程需要分开调用 plan、plan-review、start、code-review；runtime 文档包含 `spec.md`、`tasks.md`、`plan.html` 等较重模板。
- 实现后: `tick` 可以一次派发所有 eligible issue-agent，不等待它们结束；runtime 文档简化为 request、plan、change-doc、journal、status 和 review details。一个 issue 使用一个连续子 session 保留上下文，blocked 后通过 recovery 文档恢复。

## 关键设计点

### 一个 Issue 一个连续 Agent

不再拆 plan-agent 和 implementation-agent。issue-agent 在同一个 Codex CLI 子运行中先完成 planning review，再进入 implementation review。上下文保留，但状态和恢复依赖文件，不依赖聊天记忆。

### 文档驱动恢复

`agent-journal.md` 记录每轮做了什么，`status.json` 记录当前阶段和阻塞原因，`recovery.md` 在 blocked 时生成。新 session 只需要读这些文件和 review summary，就能快速接手。

### 批量异步 Tick

`tick` 先刷新既有 agent 的 exit code 和 approval 状态，再为全部 eligible request 准备文档包并异步启动 `tools/issue-agent.sh`。每个 agent 的 pid、stdout、stderr 和 exit code 写入 `.codex-auto-dev/state/agents/`，下一次 tick 根据这些文件把 request 推进到 `waiting-finish` 或 `blocked`。

### 简洁 Runtime 文档

`plan.md` 合并 spec、plan 和 tasks，并在顶部固定记录平台无关的规范化需求信息: request ID、external ID、source、URL、需求名称和需求描述。`change-doc.md` 汇总实现说明和最终 review 结果；review 原始 JSON 放在 details 中作为机器证据。

### Connector Contract

默认 `issue-update`、`issue-agent`、reviewer 和 PR connector 都写明输入输出契约。issue-update 的 stdout 统一为 `external_id<TAB>source<TAB>title<TAB>body<TAB>url`，reviewer stdout 统一为结构化 JSON，PR connector 成功时 stdout 只输出 PR URL。这样 issue-agent prompt 可以保持通用，不绑定 GitHub 或内部平台。

## 变更范围摘要

预计改动集中在 CLI 命令、runtime 模板、review 输出路径、默认 issue-agent connector、README、skill 和集成测试。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`

## 风险与后续

- 后续可以新增 `tick --interval 15m`，但第一版建议由 Codex heartbeat 或系统 cron 调用 `tick`。
- 后续可以增加更强的并发状态锁和跨 issue 冲突检测。
