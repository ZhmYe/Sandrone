# 变更文档: CodeGraph Auto Init

## 摘要

本次变更让框架在非空目标仓库中自动尝试初始化 CodeGraph 索引，减少 Codex 调用 CodeGraph MCP 时遇到 “not initialized” 的情况。同时明确 `.codegraph` 索引目录和 `docs/codegraph/context.md` 架构文档的职责边界。

## 实现前后对比

- 实现前: `new --url` 只提示 CodeGraph required；`plan` preflight 只检查 `docs/codegraph/context.md` 是否缺失或过期。即使本机装了 CodeGraph，也不会自动创建 `dev/repo/.codegraph`。
- 实现后: 非空 clone 后会自动尝试 `codegraph init -i dev/repo`；计划前检查也会补一次初始化。失败或命令不存在时记录 warning，不阻断流程。

## 关键设计点

### 自动初始化但不强制阻塞

新增 `CodegraphInitOutcome` 区分空仓库跳过、已初始化、初始化成功、命令不可用和初始化失败。`new` 和 `plan` 使用这个结果输出可读提示；失败不会 panic。

### 索引和文档分离

`dev/repo/.codegraph` 是 CodeGraph MCP 的索引目录。`docs/codegraph/context.md` 是给 agent/reviewer 阅读的架构文档。CLI 负责自动尝试创建索引，但仍提示用户或 agent 通过 `codegraph-project-preview` skill 生成/刷新文档。

### Doctor 可见性

`doctor` 增加 CodeGraph CLI 和 CodeGraph index 检查。目标仓库为空时 index 检查通过；有 commit 但缺少 `.codegraph` 时显示 warning。

## 变更范围摘要

- CLI: clone 和 plan preflight 自动尝试 `codegraph init -i dev/repo`。
- Doctor: 增加 CodeGraph CLI/index。
- Git hygiene: 忽略本仓库本地 `.codegraph/` 索引目录。
- Prompt/docs: 说明 CodeGraph 生命周期。
- 测试: fake CodeGraph 覆盖 clone 和 preflight 初始化。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、CLI clone/preflight/doctor 逻辑和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 验证证据

- TDD red: `cargo test codegraph --test cli_flow` 中 preflight 测试因缺少 `preflight: CodeGraph initialized` 失败。
- TDD green: 实现后 `cargo test codegraph --test cli_flow` 通过。
- `cargo test new_url_clones_existing_target_repo --test cli_flow` 通过，确认 clone 后调用 fake CodeGraph。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，27 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 17 个 proposal。
- `git diff --check` 通过。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试和 proposal 校验作为交付证据。
