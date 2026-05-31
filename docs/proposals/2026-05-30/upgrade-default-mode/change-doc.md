# 变更文档: Upgrade Default Mode

## 摘要

本次变更把升级语义拆成两个明确模式: 普通 `upgrade` 只刷新框架维护的 `.example.*` 参考文件并提醒用户自行选择；`upgrade --default` 才把这些 example 覆盖到正式 connector、prompt 和 review schema。

## 实现前后对比

- 实现前: 普通 `upgrade` 会刷新 example，同时还会为缺失的正式 connector/prompt/schema 创建默认文件。这对旧 workspace 友好，但边界不够清楚。
- 实现后: 普通 `upgrade` 对正式运行资产完全保守，只刷新 example 和输出提醒；需要使用默认实现时，用户显式运行 `upgrade --default`。

## 关键设计点

### Target/Example 映射

新增默认运行资产映射，记录正式路径、example 路径、默认内容和是否可执行。`new`、example 刷新和 `--default` 覆盖都复用这份映射，减少模板漂移。

### 普通 Upgrade 保护边界

普通 `upgrade` 继续迁移 schema version、session registry、approval 目录、runtime 文档和 skill 副本，但不创建或覆盖正式 `tools/*.sh`、`tools/prompts/*.md`、review schema。

### Default 覆盖模式

`upgrade --default` 先刷新 `.example.*`，再从 example 复制到正式文件，并恢复脚本可执行权限。这个模式适合确认没有本地定制，或已经人工确认要全部回到框架默认实现的 workspace。

## 变更范围摘要

- CLI: `upgrade` 支持 `--default`，help 更新为 `upgrade [--dry-run] [--default]`。
- 模板: 默认资产映射统一 target/example。
- 测试: 新增/扩展 new、upgrade 和 upgrade default 行为测试。
- 文档: README 与 workflow skill 说明两个升级模式。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、现有 upgrade proposal、CLI upgrade 逻辑和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- TDD red: `cargo test upgrade_ --test cli_flow` 中两个 upgrade 测试因缺少提醒和 `--default` 替换输出失败。
- TDD green: 实现后 `cargo test upgrade_ --test cli_flow` 通过。
- `cargo test new_name_creates_framework_and_empty_target_repo_only --test cli_flow` 通过，确认 new 的正式默认文件与 example 内容一致。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，26 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 16 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试和 proposal 校验作为交付证据。
