# PR Conflict Attempt Records Plan

## 目标顺序

1. 收窄 `pr-refresh` 记录逻辑，只在 rebase 冲突分支生成独立 attempt 文件。
2. 将冲突 attempt 存放到 `docs/changes/<name>/pr-conflicts/attempts/NNN-rebase-conflict.md`。
3. 在 `change-doc.md` 中追加 `PR 冲突记录`，保留冲突诊断和 attempt 文件路径。
4. 保持 clean rebase 与 rebase agent 完成后的 `PR 集成刷新记录` 不变。
5. 让 dashboard 的 PR refresh 判断同时识别 `PR 冲突记录` 和 `pr-conflicts/attempts`。
6. 补充测试和文档，明确多次冲突只记录真实冲突。

## 关键设计

- 使用文件名三位编号保证同一 request 的多次冲突可排序、可审计。
- 独立冲突记录保留原始诊断，`change-doc.md` 只挂摘要和路径，避免页面被大量诊断淹没。
- 不为非冲突刷新写 attempt，避免把普通 rebase 操作误读成冲突处理。

## 测试

- `cargo test pr_refresh_conflict_uses_rebase_agent_and_integration_review`
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`
- `cargo fmt --check`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
