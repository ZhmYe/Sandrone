# 变更文档: Runtime Status Sync And Detailed Review Findings

## 摘要

本次变更修复了中央 `requests.tsv` 落后于 runtime `status.json` 时可能重复 plan-review/start/implementation 的问题，并把 reviewer finding 输出升级为更可执行的结构化建议。

## 实现前后对比

- 实现前: `advance` 直接按 `requests.tsv` 的 `plan-submitted` 判断下一步。如果该索引落后，而 `status.json` 已经进入 `change-doc-submitted`，流程会重新跑 plan-review 和 start，可能派发重复 implementation。
- 实现后: `advance` 和 `tick` 处理 request 前先读取 `status.json`，只把更靠后的 runtime 状态同步回 `requests.tsv`。同样场景会直接进入 code-review。
- 实现前: reviewer finding 只要求 `title/evidence/required_fix`，拒绝意见可能不够具体。
- 实现后: finding 必须包含 `impact/required_fix/suggested_change/verification`，拒绝时每条问题都带具体修改建议和验证方式。

## 关键设计点

### Runtime 状态同步

新增 `sync_request_from_status_json`。它验证 `status.json.request_id` 与 request 匹配，并用状态 rank 判断 runtime 状态是否比中央状态更靠后。只有推进时才同步，避免旧 `status.json` 覆盖新中央状态。branch/worktree 只在中央字段为空时补齐。

### 幂等推进

同步发生在 `refresh_request_status_by_id` 和 `dispatch_next_agent_for_request` 的入口。这样无论来自 heartbeat 还是 hook 的 `advance`，都会先修复 stale index，再判断 plan-review、code-review 或 agent dispatch。

### 详细 Reviewer Finding

review schema 的 finding 增加 `impact`、`suggested_change` 和 `verification`。fallback JSON 和默认 reviewer prompt 示例同步更新。缺少这些字段的 reviewer 输出会被转换为 blocking `gate_unavailable`，防止含糊 review 推动自动流程。

## 变更范围摘要

- 状态机: 从 `status.json` 同步更靠后的 runtime 状态。
- Review schema: finding 增加强制字段。
- Prompt/docs: 更新 reviewer 输出协议和示例。
- 测试: 增加 stale index 和 detailed finding 回归测试。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、advance/tick 状态机、review schema 和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- TDD red: `cargo test advance_syncs_stale_request_index_from_status_json_before_reviewing --test cli_flow` 因 PlanReviewer 被重复运行而失败。
- TDD green: 实现 runtime sync 后该测试通过。
- TDD red: `cargo test review_gate_rejects_findings_without_detailed_modification_advice --test cli_flow` 因旧 finding 被普通 rejected 接受而失败。
- TDD green: 扩展 schema/fallback 后该测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，29 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 18 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试和 proposal 校验作为交付证据。
