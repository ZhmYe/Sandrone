# 任务: Strict Reviewer Gates

## 实现任务

- [x] 添加 reviewer asset 创建测试。
- [x] 添加 plan-review/code-review gate 测试。
- [x] 新增 reviewer constants 和默认脚本/prompt/schema。
- [x] 新增 `plan-review` 命令。
- [x] 新增 `code-review` 命令。
- [x] 写入 reviewer JSON 和 summary JSON。
- [x] high/critical 阻断 approval。
- [x] 全部 reviewer 通过后自动写 approval。
- [x] 更新 README 和 skill。
- [x] 更新 proposal index。

## 验证

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
