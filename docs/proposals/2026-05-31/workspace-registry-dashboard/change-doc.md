# Workspace Registry Dashboard Change Doc

## 摘要

本次变更为框架增加全局 workspace registry 和本地 dashboard。用户可以通过 `codex-auto-dev dashboard` 或 `cad dashboard` 在浏览器中查看所有已登记项目，并按项目、需求和 stage 查看关键文件与 review detail。

## 实现前后对比

变更前:

- CLI 只能在当前 workspace 内 `list/status`。
- 没有全局 workspace 发现机制。
- 没有浏览器前端。
- review 多轮 detail 只能手动进入目录查看。

变更后:

- `new`、`upgrade`、`list`、`dashboard` 会维护全局 `workspaces.json`。
- `dashboard --json` 提供稳定数据模型。
- `dashboard` 启动本地 HTTP 前端。
- 前端左侧展示项目，右侧展示 request 列表、6 段 timeline 和选中 stage 的核心文件。
- review stage 按 attempt 展示每轮 reviewer detail，不依赖 `summary.json`。
- request 列表改为纵向扫描布局，文件内容使用 Markdown/JSON 专用渲染库美化展示。
- 新增 `cad` 作为短命令别名。

## 关键设计点

- Registry 位置默认 `~/.codex-auto-dev/workspaces.json`，测试和机器人环境可用 `CODEX_AUTO_DEV_HOME` 隔离。
- Dashboard 从 registry 刷新每个 workspace，缺失路径标记为 `missing`，避免一个旧路径破坏整个页面。
- 普通 stage 映射到一个核心文件，review stage 映射到 `details/*.json`。
- Markdown 使用 `marked` 渲染并通过 `DOMPurify` 清洗，再用 `highlight.js` 高亮代码块。
- JSON 和 reviewer detail 使用 `jsoneditor` 只读 view 模式；CDN 不可用时退回纯文本。
- `cad` 使用包装二进制转调同目录下的 `codex-auto-dev`，避免复制主 CLI 实现。
- 前端为只读页面，后续审批/finish 等交互可以在同一 API 层扩展。

## 变更范围摘要

- `src/main.rs`: 新增 registry、dashboard API、HTTP server、dashboard HTML 和 CLI 命令接入。
- `src/bin/cad.rs`: 新增短命令包装器。
- `Cargo.toml`: 新增 `cad` bin。
- `tests/cli_flow.rs`: 新增 registry、dashboard JSON 和别名测试。
- `README.md`、`skills/codex-auto-dev-workflow/SKILL.md`: 更新使用说明。
- `.gitignore`: 忽略本地视觉草图目录。

## 验证证据

- 已完成局部红绿测试:
  - `cargo test workspace_registry_tracks_new_upgrade_and_current_list_refresh --test cli_flow`
  - `cargo test dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts --test cli_flow`
  - `cargo test cad_alias_prints_the_same_cli_help --test cli_flow`
  - `cargo test help_lists_state_and_validation_commands --test cli_flow`

完整验证将在最终交付前运行并回填结果。

## Review 结果

本次为框架源码变更，最终 review 以本仓库测试、clippy 和 proposal 校验为准。
