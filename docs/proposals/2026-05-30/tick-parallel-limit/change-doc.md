# 变更文档: Tick Parallel Limit

## 摘要

本次变更为 `sandrone tick` 增加自动 issue 处理的并发上限。新 workspace 默认 `parallel_limit = 1`，也就是同一时间最多自动处理 1 个 issue；需要并行时可以修改配置或运行 `sandrone tick --parallel-limit <N>` 单次覆盖。

## 实现前后对比

- 实现前: `tick` 会扫描所有 eligible request 并全部派发 agent。多 issue 时容易同时启动过多 Codex 子运行。
- 实现后: `tick` 先刷新状态，再统计 running request，只按剩余槽位派发新 agent。达到上限时 pending request 保持等待，下一次 tick 再处理。

## 关键设计点

### 配置默认值

`Config` 新增 `parallel_limit`。新 workspace 写入 `parallel_limit = 1`；旧 workspace 读取不到该字段时默认使用 1，upgrade 重写 config 时会补上字段。

### Tick 调度

`tick` 支持 `--parallel-limit` 和 `--parallel_limit`。命令行参数优先于 config。主控在 `refresh_tick_statuses` 后统计 `planning-agent-running`、`implementation-agent-running` 和 legacy `agent-running`，这些 request 都占用并发槽。

### 可观察输出

如果 running 数量已经达到上限，`tick` 输出 `Tick parallel limit reached: X/Y issue-agent(s) already running.`。如果只派发了部分 pending request，输出剩余数量，方便 heartbeat log 和未来前端展示。

## 变更范围摘要

- CLI: `tick [--parallel-limit <N>]`。
- Config: `.sandrone/config.toml` 增加 `parallel_limit = 1`。
- 状态机: tick 按剩余并发槽位截断派发列表。
- 测试与文档: 覆盖默认、覆盖、非法参数和运行中占槽行为。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、tick/advance 状态机、集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 文档与 Checklist

- 已更新的文档: README、workflow skill、本 proposal。
- 所有交付文档中的 checklist 是否已全部打勾: yes；检查范围包括本 proposal 的 `tasks.md`、本 `change-doc.md`、README 和 workflow skill。
- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: yes；本次没有剩余人工事项。

## 后续流程

本次没有需要保留的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。

## 验证证据

- TDD red: `cargo test --test cli_flow tick_default_parallel_limit_counts_running_issue_agents -- --nocapture` 失败，因为旧逻辑会派发两个 request。
- TDD green: 实现并发槽位后，该测试通过。
- `cargo test --test cli_flow tick_parallel_limit_flag_dispatches_multiple_pending_issue_agents_without_waiting -- --nocapture` 通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，31 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 20 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
