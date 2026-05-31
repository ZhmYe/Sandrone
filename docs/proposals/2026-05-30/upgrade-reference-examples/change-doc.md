# 变更文档: Upgrade Reference Examples

## 摘要

本次变更让 `upgrade` 在保护用户正式脚本的同时，刷新一套框架维护的 `.example.*` 参考文件。旧 workspace 可以通过这些 example 对比新版默认 connector、prompt 和 review schema，再由用户决定是否复制到正式文件。

## 实现前后对比

- 实现前: `upgrade` 只在正式 connector 缺失时创建默认文件。用户已有脚本会被保护，但也无法安全看到新版默认脚本和 schema。
- 实现后: 正式 `tools/*.sh`、`tools/prompts/*.md` 和 `tools/schemas/review-result.schema.json` 仍然不被覆盖；`upgrade` 会刷新 `*.example.sh`、`*.example.md` 和 `*.example.schema.json`，供参考、测试和手动复制。

## 关键设计点

### 正式文件保护不变

`write_default_*` 仍然先检查正式路径是否存在，存在时直接跳过。这保证用户接入 GitHub、Jira、公司内部系统或自定义 reviewer backend 的脚本不会因为升级被替换。

### Example 由框架维护

新增 `refresh_default_reference_examples()`，每次写入框架维护的 `.example.*` 文件。example 不是运行入口，因此允许被升级覆盖；用户如果要定制，应复制到无 `.example` 后缀的正式文件。

### 默认内容共享

默认 issue connector、issue-agent、PR connector、reviewer connector 和 review schema 被抽成内容函数。正式默认文件和 example 文件共用这些函数，避免未来只更新一边造成漂移。

## 变更范围摘要

- CLI: `new` 与 `upgrade` 生成 reference examples；`upgrade --dry-run` 显示将刷新的 example 路径。
- 模板: 新增脚本、prompt 和 schema example。
- 文档: README 与 workflow skill 说明 `.example.*` 的用途和边界。
- 测试: 扩展 upgrade 集成测试，覆盖正式文件保护和 example 刷新。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、现有 upgrade 测试、默认 connector 实现。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- TDD red: `cargo test upgrade_migrates_old_workspace_without_overwriting_user_connectors` 因缺少 `Would refresh tools/issue-update.example.sh` 失败。
- TDD green: 实现后同一测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，25 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试和 proposal 校验作为交付证据。
