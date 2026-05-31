# 任务: Clone CodeGraph And Finish Delivery

## 测试先行

- [x] 添加空仓库 clone 跳过 CodeGraph 测试。
- [x] 添加有内容仓库 clone 要求 CodeGraph 测试。
- [x] 添加 plan 前发现 upstream 更新并失败的测试。
- [x] 更新 finish 测试，要求审批后 commit/push 到 request 分支。

## 实现

- [x] `new --url` 输出空仓库/CodeGraph 判断。
- [x] `plan` 前检查 upstream 是否需要 pull。
- [x] `plan` 前检查是否需要刷新 CodeGraph。
- [x] 将 preflight 写入计划模板和 handoff。
- [x] `finish` 支持 `--message`。
- [x] `finish` 执行 commit/push。
- [x] `finish` 通过 `tools/pr-create.sh` 尝试 PR 或输出 fallback。
- [x] PR body 包含 request/issue 关联信息，并为默认 GitHub issue 写入 closing keyword。
- [x] 更新 skill、README、constitution 和 proposal。

## 验证

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
