# 任务: Tick Issue Agent

## 实现任务

- [x] 更新 runtime 文档包测试。
- [x] 添加 issue-agent asset 测试。
- [x] 添加 `tick` 派发测试。
- [x] 添加 `block` / `resume` 测试。
- [x] 添加 review summary 写入 change-doc 测试。
- [x] 新增 issue-agent constants 和默认脚本/prompt。
- [x] 修改 `plan` 生成简化文档包。
- [x] 新增 `tick` 命令。
- [x] 新增 `block` 命令。
- [x] 新增 `resume` 命令。
- [x] 修改 review details/summary 路径。
- [x] 修改 PR body 不依赖 tasks。
- [x] 在 `plan.md` 中加入规范化需求记录。
- [x] 为可替换脚本补充输入输出 contract。
- [x] 更新 README 和 skill。
- [x] 更新 proposal index。

## 验证

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`
- [x] `git diff --check`
