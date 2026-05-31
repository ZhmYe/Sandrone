# 变更文档: Resumable Blocked Requests And Codex Bin Resolution

## 摘要

本次修复两个自动流程恢复问题: 默认 connector 不再依赖写死的 Codex.app 路径，而是通过环境变量或 PATH 解析 Codex CLI；`resume` 现在会把 blocked request 真正恢复成 tick 可派发状态。

## 实现前后对比

- 实现前: 默认 `issue-agent.sh` 和 reviewer scripts 直接调用 `codex exec`，普通终端 PATH 没有 codex 时会失败。
- 实现后: 默认 connector 使用 `resolve_codex_bin`，优先读取 `CODEX_AUTO_DEV_CODEX_BIN`，其次查 PATH，最后使用用户显式提供的 `CODEX_AUTO_DEV_CODEX_APP` bundle。
- 实现前: `resume` 只打印 request、plan、change-doc、recovery 等路径，`requests.tsv` 和 `status.json` 仍是 `blocked`。
- 实现后: `resume` 对 blocked request 会按 approval 状态恢复为 `planning` 或 `in-progress`，同步状态文件、session 和事件流，后续 `tick --request_id` 可以派发。

## 关键设计点

### 不写死本机路径

默认脚本不包含 `/Applications/Codex.app`。如果用户从普通终端运行，需要显式设置 `CODEX_AUTO_DEV_CODEX_BIN` 或 `CODEX_AUTO_DEV_CODEX_APP`，或者把 `codex` 放进 PATH。这样框架可以跨安装位置、跨平台和跨后端替换。

### Reviewer Gate 失败仍然安全

agent connector 找不到 Codex CLI 时非 0 退出；reviewer connector 找不到 Codex CLI 时输出结构化 `gate_unavailable=true`。两者都不会伪造通过。

### Resume 状态恢复

`resume` 不删除 recovery 文档或历史 review。它只把 terminal `blocked` 状态改回可调度状态，并根据 plan approval 决定下一次 tick 应进入 planning 还是 implementation。

## 变更范围摘要

- CLI: `resume` 更新状态文件、session 和事件流。
- 默认脚本: issue-agent 和 reviewer scripts 支持可配置 Codex CLI 解析。
- 测试: 默认脚本生成测试、blocked request resume 派发测试。
- 文档: README、workflow skill、本 proposal。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、resume/tick/default connector 代码和集成测试。
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

- TDD red: `cargo test --test cli_flow new_name_creates_framework_and_empty_target_repo_only -- --nocapture` 失败，默认脚本缺少可配置 Codex CLI 解析。
- TDD red: `cargo test --test cli_flow block_and_resume_create_recovery_package -- --nocapture` 失败，`resume` 没有输出 `resumed status`，状态仍无法派发。
- TDD green: 实现 `resolve_codex_bin` 和 resume 状态恢复后，上述两个测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，34 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 24 个 proposal。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
