# 任务: Explicit Approval, Thread Registry And Upgrade

## 测试先行

- [x] 添加 `start_requires_plan_approval_before_creating_worktree`。
- [x] 添加 `finish_requires_change_doc_approval_without_commit_or_push`。
- [x] 添加 `approval_becomes_stale_when_artifact_changes_after_approval`。
- [x] 添加 `session_command_registers_visible_thread_links`。
- [x] 添加 `upgrade_migrates_old_workspace_without_overwriting_user_issue_tool`。
- [x] 更新计划模板测试，要求中文模板与 handoff 上下文。

## 实现

- [x] 新增 approval 命令与 JSON 文件。
- [x] 新增 artifact hash 校验与 stale 检查。
- [x] 将 `start` 和 `finish` 接入强制门禁。
- [x] 新增 session registry 与登记命令。
- [x] 新增 `upgrade --dry-run` 和 `upgrade`。
- [x] 将 runtime 模板和 handoff 改为中文。
- [x] 调整 change-doc 模板，强调实现前后对比、关键设计点和变更范围摘要。
- [x] 更新 skill 和 README。
- [x] 新增 proposal 文档并更新索引。

## 验证

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
