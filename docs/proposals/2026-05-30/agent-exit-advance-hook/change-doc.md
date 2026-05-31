# 变更文档: Agent Exit Advance Hook

## 摘要

本次变更新增 `advance` 单 request 推进命令，并在 agent wrapper 写入 exit code 后立即调用它。这样 planning agent 完成后不必等下一次 heartbeat，就能进入 plan-review、start 和 implementation；implementation 完成后也能立刻进入 code-review 并停在 `waiting-finish`。

## 实现前后对比

- 实现前: `tick` 派发 agent 后立即返回；agent 完成后只写 exit code，必须等下一次 `tick` 才会刷新状态和执行 reviewer。
- 实现后: wrapper 在 agent 退出时写 exit code，并把 `codex-auto-dev advance --request_id <REQ>` 的输出写入 `.codex-auto-dev/state/agents/<REQ>.hook.log`。heartbeat 仍保留，负责发现新 issue 和兜底恢复。

## 关键设计点

### Advance 命令

`advance` 不运行 `update`，也不扫描全部 request。它只处理一个 request:

- 刷新 agent exit code。
- planning agent 成功时提交 plan gate 并运行 PlanReviewer。
- plan-review 通过后创建 worktree 并派发 implementation agent。
- implementation agent 成功时提交 change-doc gate 并运行 TestReviewer 和 DesignReviewer。
- code-review 通过后标记 `waiting-finish`。
- reviewer rejected 时派发下一轮对应 phase，超过 max attempts 后 block。

### Wrapper Hook

Rust 的 `spawn_issue_agent` 本来就有一层 shell wrapper 负责等待 `tools/issue-agent.sh` 并写 exit code。本次在同一层追加 hook: 写 exit code 后调用 `advance`。这不是 Git hook，也不是 Codex 内置 hook，而是框架自己的 post-exit hook。

### Per-request Lock

`advance` 和 `tick` 刷新/派发 request 时都会尝试创建 `.codex-auto-dev/state/locks/<request_id>.lock/`。拿不到锁说明已有 hook 或 heartbeat 正在处理该 request，当前进程直接跳过。lock 写入 pid，发现 pid 已不存在时会清理 stale lock。

## 变更范围摘要

- CLI 命令: 新增 `advance`。
- 状态机: 抽出单 request 派发 helper，tick 与 advance 共用。
- Agent wrapper: 写 exit code 后调用 advance。
- Runtime state: 新增 hook log 和 request lock 目录。
- 文档/skill: 更新 heartbeat 与 hook 的职责说明。
- 测试: 新增一次 tick 后自动推进到 waiting-finish 的集成测试。

## 目标项目内部要求

- 已阅读的目标项目文档: README、skill、现有 tick/review 测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py` 通过。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 均通过。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- TDD red: `cargo test agent_exit_hook_advances_request_without_waiting_for_next_tick -- --nocapture` 在实现前失败，等待 implementation phase 日志超时。
- TDD green: 同一测试在实现后通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，22 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 13 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；验证依赖本地格式、编译、clippy、测试和 proposal 校验。
