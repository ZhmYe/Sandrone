# Upgrade Preserve Agent Journal Change Doc

## 摘要

本次修复 `upgrade` 会覆盖 `agent-journal.md` 的问题。journal 是恢复自动流程的重要审计文件，普通升级必须保留历史记录，而不是把它当作可刷新的模板。

## 实现前后对比

变更前:

- `should_write_managed_artifact` 只要看到 `agent 每轮` 就认为文件仍是模板。
- 新建 journal 的默认说明本身就包含这句话。
- 因此运行 `upgrade` 会重写所有仍带默认说明的 journal，即使后面已经追加过真实 attempt。

变更后:

- `agent-journal.md` 进入文件名级别特殊处理。
- 已存在且包含正常历史内容的 journal 不再被覆盖。
- 只有空文件、缺失文件或旧 handoff/prompt 文档才会被迁移。
- 回归测试构造包含默认说明和 `Attempt 1 - planning` 的 journal，确认升级后内容保留。

## 验证证据

- `cargo test upgrade_refreshes_examples_without_overwriting_user_connectors`: 通过。
- `cargo test`: 通过，43 个 CLI flow 测试通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，validated 41 proposal(s)。
- `git diff --check`: 通过。
