# PR Conflict Attempt Records Change Doc

## 摘要

本次把 PR refresh 的 attempt 记录收窄为“只记录真实 PR 冲突”。clean rebase、merged skip 和普通 continue 不再生成 attempt 文件；发生 rebase 冲突时，会在 request 文档包下写入独立冲突记录，并在 `change-doc.md` 中追加摘要。

## 实现前后对比

变更前:

- 集成刷新记录只集中在 `change-doc.md`。
- 如果同一个 PR 后续再次冲突，缺少独立、可排序的冲突诊断文件。

变更后:

- `append_integration_record` 只负责普通 `PR 集成刷新记录`。
- rebase 冲突分支调用 `append_pr_conflict_record`。
- 冲突 attempt 写入 `pr-conflicts/attempts/NNN-rebase-conflict.md`。
- `change-doc.md` 追加 `PR 冲突记录 (Attempt NNN)`，包含记录路径、base ref、HEAD 和诊断摘要。
- dashboard 仍显示简洁支线，不新增 attempt UI，但可以识别冲突记录存在。

## 验证证据

- `cargo test pr_refresh_conflict_uses_rebase_agent_and_integration_review`: 通过。
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`: 通过。
- `cargo test`: 通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `cargo fmt --check`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，validated 40 proposal(s)。
- `git diff --check`: 通过。

## 后续流程

- 完成本轮全量验证后，再根据需要安装本地 skill/CLI 或提交推送。
