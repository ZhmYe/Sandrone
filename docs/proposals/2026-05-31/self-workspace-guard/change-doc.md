# Self Workspace Guard Change Doc

## 摘要

本次变更清理了框架源码仓库根目录中误生成的 managed workspace 运行态目录，并新增 `sandrone new` 自保护。以后在框架源码 checkout 根目录运行 `new` 会直接失败，不会再创建 `.sandrone/`、`dev/` 或 `tools/`。

## 实现前后对比

变更前:

- 在框架源码仓库根目录运行 `sandrone new` 会把当前仓库初始化成 managed workspace。
- 根目录会出现 `.sandrone/`、`dev/` 和 `tools/`。
- 这些目录虽被忽略，但会污染本地 checkout。

变更后:

- `new_workspace` 入口会识别 `sandrone` 源码 checkout。
- 检测命中时返回明确错误，提示切换到单独外层目录。
- 集成测试确认失败时不创建运行态目录。

## 关键设计点

- 检测条件保守组合 `Cargo.toml` package name、`src/main.rs`、`templates/` 和 `skills/sandrone/`，避免误伤普通 Rust 项目。
- 保护只作用于 `new`，不影响正常 managed workspace 的 `update/tick/upgrade`。
- 清理只删除误生成的 managed workspace 运行态目录和 `.DS_Store`，保留 `target/`、`.codegraph/`、`.omx/` 等开发缓存。

## 变更范围摘要

- `src/main.rs`: 新增源码 checkout 检测和 `new` 入口保护。
- `tests/cli_flow.rs`: 新增自保护集成测试。
- `README.md`、`skills/sandrone/SKILL.md`: 增加使用说明。
- 本地清理: 删除 `.sandrone/`、`dev/`、`tools/` 和 `.DS_Store`。

## 验证证据

- `cargo test new_refuses_to_initialize_the_framework_source_checkout`: 先失败于误创建 workspace，添加保护后通过。
- `cargo fmt --check`: 通过。
- `cargo check`: 通过。
- `cargo test`: 通过，1 个单元测试和 42 个 CLI 集成测试通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，校验 32 个 proposal。
- `git diff --check`: 通过。
- `test ! -e dev && test ! -e .sandrone && test ! -e tools`: 通过，确认源码仓库根目录已清理。

## Review 结果

本次是框架自保护修复，没有运行自动 reviewer gate；以本仓库测试、clippy、proposal 校验和人工检查作为交付前验证。
