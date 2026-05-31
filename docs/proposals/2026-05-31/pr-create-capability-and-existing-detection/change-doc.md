# 变更文档: PR Create Capability And Existing Detection

## 摘要

本次变更让 PR connector 在创建前必须判断平台能力并检查已有 PR。`finish` 现在能区分新建 PR 和已有 PR，避免重复创建或误导用户。

## 实现前后对比

- 实现前: 默认 `pr-create.sh` 直接调用 `gh pr create`，如果平台不支持、gh 不可用或 PR 已存在，错误依赖平台输出。
- 实现后: 默认脚本先检查 `gh` 和仓库可访问性，再检查 base/head 是否已有 PR；已有时输出 `existing<TAB>url`，没有时才创建并输出 `created<TAB>url`。
- 实现前: Rust 只把 stdout 当作 PR URL，无法区分已有 PR。
- 实现后: Rust 解析 `created<TAB>url`、`existing<TAB>url` 和旧裸 URL，分别输出 `PR created` 或 `PR already exists`。

## 关键设计点

### 平台中立 Contract

`tools/pr-create.sh` 仍是可替换 connector。GitHub 只是默认实现；其他平台可以用同样 TSV 输出表达 created/existing。无法判断平台能力或已有 PR 时，connector 必须失败并输出清晰 stderr。

### 幂等 PR 交付

默认 GitHub 脚本使用 `gh pr list --state all --base ... --head ...` 检查已有 PR。这样 repeated finish/recovery 不会直接再次创建。

### 向后兼容

旧 connector 只输出 URL 时，`finish` 仍按 created 处理，避免破坏已有 workspace。

## 变更范围摘要

- CLI: `finish` 识别 PR connector 的 created/existing 状态。
- 默认脚本: `tools/pr-create.sh` 增加能力检查和已有 PR 检查。
- 文档: README、workflow skill、proposal。
- 测试: 默认脚本内容断言和 existing PR finish 测试。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、finish/PR delivery 代码和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 文档与 Checklist

- 已更新的文档: README、workflow skill、本 proposal。
- 所有交付文档中的 checklist 是否已全部打勾: yes；检查范围包括本 proposal 的 `tasks.md`、本 `change-doc.md`、README 和 workflow skill。
- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: yes；本次没有剩余人工事项。

## 后续流程

本次没有需要保留的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。

## 验证证据

- TDD red: `cargo test --test cli_flow new_name_creates_framework_and_empty_target_repo_only -- --nocapture` 失败，默认脚本缺少新 contract。
- TDD red: `cargo test --test cli_flow finish_reports_existing_pr_from_pr_connector -- --nocapture` 失败，`finish` 未识别 existing PR。
- TDD green: 更新解析和默认脚本后，上述两个测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，34 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 22 个 proposal。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
