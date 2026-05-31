# Assets Template Boundary Plan

## 目标与依赖顺序

1. 移动 dashboard HTML 到 `assets/dashboard/index.html`。
2. 更新 `src/assets.rs` 的 `DASHBOARD_HTML` include 路径。
3. 更新结构测试，从 `assets/dashboard/index.html` 检查 dashboard 页面。
4. 更新 README、Skill 和 source template refactor 文档。
5. 运行 dashboard 相关测试和全量验证。

## 设计规则

- `assets/`: 固定静态资产，编译进二进制，不复制到 workspace。
- `templates/`: 会被填充、复制、升级为 `.example` 或作为默认可替换内容的模板。

## 验证

- `cargo test templates_are_external_assets_not_embedded_in_main`
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
