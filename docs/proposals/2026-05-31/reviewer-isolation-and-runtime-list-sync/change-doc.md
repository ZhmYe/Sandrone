# 变更文档: Reviewer Isolation And Runtime List Sync

## 摘要

本次变更让 code-review 的 TestReviewer 和 DesignReviewer 在隔离上下文中独立评审，并修复 `list/status` 直接读取滞后 TSV 导致状态显示过旧的问题。

## 实现前后对比

- 实现前: reviewer 环境中的 `SANDRONE_CHANGE_PATH` 指向原始 change 目录。历史 `reviews/`、当前轮 TestReviewer detail、历史 summary 都可能被后续 reviewer 读取。
- 实现后: 每个 reviewer 拿到独立 `SANDRONE_REVIEW_CONTEXT`，其中没有 `reviews/` 或 agent journal。canonical review detail 仍由框架写回原始 review 目录。
- 实现前: `list/status` 直接 `load_requests()`，当 `status.json` 已是 `waiting-finish` 而 TSV 仍是 `implementation-agent-running` 时，用户会看到旧状态。
- 实现后: `list/status` 输出前运行 runtime sync，只将更靠后的 runtime 状态同步回 TSV。

## 关键设计点

### Reviewer 隔离上下文

新增 `prepare_review_context`，为每个 reviewer 和 attempt 创建独立目录，只复制 `request.md`、`plan.md`、`change-doc.md`、`status.json` 和 `approvals/`。不复制 `reviews/`，也不复制 `agent-journal.md`，避免历史 reviewer finding 间接影响独立评审。

### Reviewer 环境变量

`SANDRONE_CHANGE_PATH` 现在指向隔离 context；`SANDRONE_REVIEW_CONTEXT` 明确标识该目录；`SANDRONE_CANONICAL_CHANGE_PATH` 保留 canonical change 目录位置；`SANDRONE_REVIEW_FORBIDDEN_PATHS` 声明 reviewer 不得读取的原始 review 路径。

### 观察入口同步

新增 `sync_all_requests_from_status_json`，在 `list` 和 `status` 输出前同步所有可推进的 runtime 状态。它复用已有 rank 规则，不允许 runtime 状态回退中央索引。

## 变更范围摘要

- Review runner: per-reviewer isolated context。
- 默认 reviewer connector/prompt: 独立评审边界和 forbidden paths。
- CLI 观察入口: `list`、`status` runtime sync。
- 测试与文档: 新增回归测试和规则说明。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、review runner、runtime status sync、集成测试。
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

- TDD red: `cargo test --test cli_flow code_reviewers_get_isolated_context_without_other_or_historical_review_outputs -- --nocapture` 失败，原因是 reviewer 环境没有隔离 context。
- TDD red: `cargo test --test cli_flow list_and_status_sync_stale_request_index_from_status_json -- --nocapture` 失败，原因是 `status` 仍显示 stale TSV 状态。
- TDD green: 实现 isolated context 和 list/status runtime sync 后，上述两个测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，33 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 21 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
