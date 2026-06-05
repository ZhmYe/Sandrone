# Self Workspace Guard Plan

## 目标与依赖顺序

1. 确认根目录污染来源，并清理 `.sandrone/`、`dev/`、`tools/` 和 `.DS_Store`。
2. 增加集成测试，模拟框架源码 checkout，在其中运行 `sandrone new`，要求失败且不创建运行态目录。
3. 在 `new_workspace` 入口增加源码 checkout 检测。
4. 更新 README 和 Skill 说明。
5. 运行全量验证。

## 实现方式

源码 checkout 检测使用保守条件:

- 当前目录存在 `Cargo.toml`，且 package name 是 `sandrone`。
- 当前目录存在 `src/main.rs`。
- 当前目录存在 `templates/`。
- 当前目录存在 `skills/sandrone/`。

满足以上条件时，`new` 返回明确错误，提示切换到单独外层目录。

## 验证

- `cargo test new_refuses_to_initialize_the_framework_source_checkout`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
