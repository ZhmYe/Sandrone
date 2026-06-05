# Assets Template Boundary Change Doc

## 摘要

本次变更明确 `assets/` 和 `templates/` 的边界。dashboard HTML 从 `templates/dashboard/index.html` 移到 `assets/dashboard/index.html`，因为它是框架内置静态页面，不是 workspace 模板。

## 实现前后对比

变更前:

- dashboard HTML 位于 `templates/dashboard/index.html`。
- `templates/` 同时包含可替换模板和固定静态页面，语义混杂。

变更后:

- dashboard HTML 位于 `assets/dashboard/index.html`。
- `templates/` 仅保留 prompt、script、runtime Markdown 和 schema 等模板内容。
- `src/assets.rs` 继续统一通过 `include_str!` 暴露编译期资产。

## 关键设计点

- 不改变 dashboard 内容和服务逻辑，只调整文件归属。
- `assets/` 表示固定静态资产，不复制到 managed workspace。
- `templates/` 表示会被填充、复制、升级为 `.example` 或作为默认可替换内容的文件。

## 变更范围摘要

- `assets/dashboard/index.html`: dashboard 页面新位置。
- `src/assets.rs`: 更新 include 路径。
- `tests/cli_flow.rs`: 更新结构测试。
- `README.md`、`skills/sandrone/SKILL.md`: 更新源码维护结构说明。
- `docs/proposals/2026-05-31/source-template-module-refactor/`: 修正文档中的 assets/templates 边界。

## 验证证据

- `cargo fmt --check`: 通过。
- `cargo test templates_are_external_assets_not_embedded_in_main`: 通过。
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`: 通过。
- `cargo check`: 通过。
- `cargo test`: 通过，1 个单元测试和 42 个 CLI 集成测试通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，校验 33 个 proposal。
- `git diff --check`: 通过。

## Review 结果

本次是目录语义调整，没有运行自动 reviewer gate；以本仓库测试、clippy、proposal 校验和人工检查作为交付前验证。
