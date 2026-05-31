# Source Template Module Refactor Plan

## 目标与依赖顺序

1. 增加结构测试，要求 `assets/`、`templates/`、`src/assets.rs`、`src/dashboard.rs` 和 `src/registry.rs` 存在，并确认 `main.rs` 不再内嵌 dashboard HTML 或 reviewer prompt 大文本。
2. 新建 `assets/` 与 `templates/` 目录；dashboard HTML 放入固定静态资产目录，runtime Markdown、prompt、script 和 schema 放入模板目录。
3. 新建 `src/assets.rs`，通过 `include_str!` 暴露静态资产和模板资产。
4. 更新默认生成函数，让它们从 `assets` 读取模板；动态部分通过小型 placeholder replacement 填充。
5. 新建 `src/dashboard.rs`，移动 dashboard HTTP server、JSON 渲染和 review artifact 映射逻辑。
6. 新建 `src/registry.rs`，移动全局 workspace registry 的读写、刷新和当前 workspace 登记逻辑。
7. 更新 README、Skill 和 proposal 索引。
8. 运行全量验证。

## 代码改动位置

- `src/main.rs`: 保留 CLI 分发和核心状态机，删除长文本模板和 dashboard/registry 具体实现。
- `src/assets.rs`: 编译期模板引用。
- `src/dashboard.rs`: dashboard HTTP/API/stage 展示。
- `src/registry.rs`: `workspaces.json` 生命周期。
- `assets/**`: 固定静态资产。
- `templates/**`: 可维护模板资产。
- `tests/cli_flow.rs`: 结构约束测试。

## 风险

- 模板外置后，如果 placeholder 拼写错误，会影响生成的 `request.md`、`plan.md` 或 connector 脚本。通过现有 `new`/`upgrade` 测试和 review schema 测试兜底。
- dashboard/registry 拆模块是纯移动，风险主要是可见性和导入错误。通过 `cargo check`、dashboard JSON 测试和 registry 测试兜底。

## 验证

- `cargo test templates_are_external_assets_not_embedded_in_main`
- `cargo test new_name_creates_framework_and_empty_target_repo_only`
- `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `python3 scripts/validate_proposals.py`
- `git diff --check`
