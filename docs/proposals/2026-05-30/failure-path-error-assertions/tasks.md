# 任务: Failure Path Error Assertions

## 实现任务

- [x] 将通用失败断言改为错误文本匹配断言。
- [x] 为 git pull 前置检查失败补充错误匹配。
- [x] 为 plan approval 缺失失败补充错误匹配。
- [x] 为 change-doc approval 缺失失败补充错误匹配。
- [x] 为 plan-review reviewer 拒绝补充 reviewer 名称匹配。
- [x] 为 code-review reviewer 拒绝补充 reviewer 名称匹配。
- [x] 为 stale approval 补充错误匹配。
- [x] 更新 constitution 的 PR gate。
- [x] 更新 proposal index。

## 验证

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`
