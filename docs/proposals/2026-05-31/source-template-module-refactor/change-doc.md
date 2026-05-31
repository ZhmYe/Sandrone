# Source Template Module Refactor Change Doc

## 摘要

本次变更把长 HTML、Markdown、prompt、shell script 和 JSON schema 从 `src/main.rs` 中移出，改为仓库内 `assets/` 与 `templates/` 资产，并拆出 `assets`、`dashboard`、`registry` 三个 Rust 模块。行为保持不变，但源码结构更容易维护和 review。

## 实现前后对比

变更前:

- `src/main.rs` 约 7800 行，同时包含业务逻辑和大量长文本模板。
- dashboard HTML、reviewer prompt、agent prompt、默认脚本和 schema 都嵌在 Rust raw string 中。
- dashboard 与 registry 逻辑和 CLI 状态机混在一个文件里。

变更后:

- `src/main.rs` 降到约 4800 行，保留 CLI 分发和核心流程。
- `assets/` 保存 dashboard 这类固定静态资产。
- `templates/` 保存可生成、可替换或会作为默认内容复制到 workspace 的 Markdown、prompt、script 和 schema。
- `src/assets.rs` 用 `include_str!` 在编译期打包模板，不增加运行时文件依赖。
- `src/dashboard.rs` 专注 dashboard HTTP 服务和数据渲染。
- `src/registry.rs` 专注全局 workspace registry。
- 新增结构测试，防止长文本重新回流到 `main.rs`。

## 关键设计点

- 模板仍编译进二进制，`cargo install` 后不要求运行时找到 `templates/` 目录。
- 动态脚本模板使用 `{{PLACEHOLDER}}` 替换，避免 Rust `format!` 和 shell/JSON 的大括号互相干扰。
- runtime Markdown 模板也使用 `{{request_id}}`、`{{title}}` 等占位符，生成逻辑保留在 Rust 中。
- dashboard/registry 先按低风险边界拆出，tick/advance 等核心状态机暂不重构。

## 变更范围摘要

- `src/main.rs`: 删除长模板和 dashboard/registry 实现，保留薄包装函数与核心流程。
- `src/assets.rs`: 新增静态资产和模板资产入口。
- `src/dashboard.rs`: 新增 dashboard 模块。
- `src/registry.rs`: 新增 registry 模块。
- `assets/**`: 新增固定静态资产。
- `templates/**`: 新增默认模板资产。
- `tests/cli_flow.rs`: 新增结构约束测试。
- `README.md`、`skills/codex-auto-dev-workflow/SKILL.md`: 补充源码维护结构说明。

## 验证证据

- `cargo test templates_are_external_assets_not_embedded_in_main`: 先失败于缺少模板文件，迁移后通过。
- `cargo test new_name_creates_framework_and_empty_target_repo_only`: 通过，确认默认 workspace 生成内容仍满足断言。
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`: 通过，确认 dashboard/registry 拆分后数据模型仍可用。
- `cargo fmt --check`: 通过。
- `cargo check`: 通过。
- `cargo test`: 通过，1 个单元测试和 41 个 CLI 集成测试通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，校验 31 个 proposal。
- `git diff --check`: 通过。

## Review 结果

本次是源码结构重构，没有运行自动 reviewer gate；以本仓库测试、clippy、proposal 校验和人工检查作为交付前验证。
