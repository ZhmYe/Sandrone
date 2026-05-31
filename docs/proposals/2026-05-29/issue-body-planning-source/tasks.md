# 任务: Issue Body Planning Source

## 实现任务

- [x] 添加默认 issue connector GET/body 测试。
- [x] 添加 title/body 进入 `issue.md` 和 planning prompt 的测试。
- [x] 修正默认 GitHub connector 为 `--method GET`。
- [x] 明确 `issue.md` 的 `需求标题` 和 `需求描述`。
- [x] 强化 `codex-plan.md`、`thread-handoff.md` 和 `tasks.md` 的要求。
- [x] 更新 README 和 skill。
- [x] 更新 proposal index。

## 验证

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
