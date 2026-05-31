# 变更文档: Worktree Start Auto Pull

## 摘要

本次变更让 `start` 在创建新的 request worktree 前自动同步目标仓库。`dev/repo` 如果可以 fast-forward，就先 `git pull --ff-only`；如果 pull 失败或仓库分叉，就 block request，避免基于过期代码创建 worktree。

## 实现前后对比

- 实现前: `start` 只运行 `git fetch`，但 worktree 仍可能基于本地旧分支创建。
- 实现后: `start` 在 worktree 不存在时先运行 `git pull --ff-only`，快进后再创建 worktree。
- 实现前: 本地和远端分叉时，仍可能创建落后或不一致的 worktree。
- 实现后: pull 失败会写入 blocked 状态、`status.json` 和 `recovery.md`，并阻止 worktree 创建。

## 关键设计点

### Fast-forward Only

使用 `git pull --ff-only`，让框架自动同步只覆盖安全快进场景。不自动 merge、不自动 rebase，避免在外框架里制造不可审计的合并结果。

### 只影响新建 Worktree

同步逻辑只在 request worktree 尚不存在时执行。已有 worktree 可能已经有 agent 改动或人工恢复内容，框架不强制重写。

### 失败即 Block

pull 失败说明基线不安全或外部状态不可用。此时 request 进入 `blocked`，后续通过 `resume` 和人工处理恢复，而不是继续创建不可靠 worktree。

## 变更范围摘要

- CLI: `start_worktree` 增加 worktree 创建前自动 pull。
- 测试: start 自动 pull 成功路径和 pull 失败 block 路径。
- 文档: README、workflow skill、本 proposal。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、start/worktree 代码和集成测试。
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

- TDD red: `cargo test --test cli_flow start_auto_pulls_target_repo_before_creating_worktree -- --nocapture` 失败，`start` 没有自动 pull。
- TDD red: `cargo test --test cli_flow start_blocks_when_target_repo_pull_fails_before_worktree_creation -- --nocapture` 失败，分叉时仍创建 worktree。
- TDD green: 实现 `git pull --ff-only` 和失败 block 后，上述两个测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，36 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 25 个 proposal。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
