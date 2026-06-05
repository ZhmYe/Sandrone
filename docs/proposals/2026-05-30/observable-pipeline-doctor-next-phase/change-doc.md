# 变更文档: Observable Pipeline Doctor And Next Phase Reviews

## 摘要

本次变更提升自动流程的可运行性、可追溯性和恢复质量。新增 `doctor` 自检命令，新增事件流 `.sandrone/state/events.ndjson`，并让 reviewer 通过 `recommended_next_phase` 明确下一轮应回 planning、implementation 或 blocked。

## 实现前后对比

- 实现前: 自动流程有 request state、status、review summary 和日志，但缺少统一事件流；环境问题需要通过失败日志反推；code-review rejected 后默认继续 implementation。
- 实现后: `doctor` 可以主动检查关键运行条件；关键状态转换追加 JSONL 事件；code-review 可以把计划层问题退回 planning，避免 implementation agent 在错误计划上反复修补。

## 关键设计点

### Doctor

`doctor` 是非破坏性检查命令。它检查 workspace 元数据、Git、Codex CLI、GitHub CLI、目标仓库、agent/reviewer connector、review schema 和 events state 目录。Codex CLI 与 GitHub CLI 缺失以 warning 呈现，避免在没有完整外部环境的测试或离线机器上 panic。

### Events

事件流使用 append-only JSON Lines，路径为 `.sandrone/state/events.ndjson`。每行包含 `time`、`event`、`request_id`、`phase`、`status` 和 `detail`。它不替代 `requests.tsv`、`status.json` 或 review summary，只提供审计与前端增量展示入口。

### Recommended Next Phase

review schema 新增 `recommended_next_phase`:

- `planning`: 回到 planning agent，修正 plan。
- `implementation`: 保持 approved plan，修代码、测试或 change-doc。
- `blocked`: 停止自动流程，等待人工恢复。

如果 code-review 任一 reviewer 推荐 planning，状态机会把 request 标为 `plan-review-rejected`，下一轮 `advance/tick` 会优先派发 planning agent，即使旧 plan approval 文件仍存在。

## 变更范围摘要

- CLI: 新增 `doctor`。
- 状态与审计: 新增事件流 helper，并在关键路径追加事件。
- Review gate: 扩展 schema、fallback JSON、summary 和下一阶段路由。
- Prompt/skill/docs: 更新 reviewer 输出协议、workflow skill 和 README。
- 测试: 集成测试从 22 个增加到 25 个。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、现有测试和 proposal 结构。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- TDD red: 新增三条测试后分别因为 unknown command、events 文件缺失、code-review 未回 planning 失败。
- TDD green: 实现后 `cargo test` 通过，25 个集成测试全部通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，25 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 14 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；验证依赖本地格式、编译、clippy、测试和 proposal 校验。
