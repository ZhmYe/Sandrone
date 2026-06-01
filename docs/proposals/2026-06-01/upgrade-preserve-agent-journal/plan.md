# Upgrade Preserve Agent Journal Plan

## 目标顺序

1. 定位 `upgrade_change_artifacts` 和 `should_write_managed_artifact` 的覆盖判断。
2. 对 `agent-journal.md` 做文件名级别特殊处理，移除对默认说明文本的覆盖触发。
3. 只允许缺失、空文件或旧 handoff/prompt 文档被迁移。
4. 增加 upgrade 回归测试，构造含默认说明和 attempt 内容的 journal。
5. 更新 README、skill 和 proposal 索引。

## 风险与兼容

- 已经被旧版本覆盖的 journal 无法从当前文件恢复；只能从日志、终端输出、备份或版本控制中人工重建。
- 新逻辑偏向保守，不会为了刷新模板而覆盖已有 journal；这符合 journal 作为审计历史的定位。

## 测试

- `cargo test upgrade_refreshes_examples_without_overwriting_user_connectors`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
