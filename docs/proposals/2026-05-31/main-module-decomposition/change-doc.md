# Main Module Decomposition Change Doc

## 摘要

本次变更将 `src/main.rs` 从 5000 多行拆分为多个 crate 内模块，保持 CLI 行为和业务流程不变。`main.rs` 现在主要承担命令入口和状态机编排，review、状态持久化、默认资产、交付、doctor 和通用工具分别进入独立文件。

## 实现前后对比

变更前:

- `main.rs` 同时包含 CLI 分发、状态读写、review runner、PR 交付、doctor、模板生成和 JSON/TSV 工具。
- 修改 review 或 dashboard 支撑逻辑时需要在单一大文件中跳转，维护成本高。

变更后:

- `src/defaults.rs` 管理 workspace 默认目录、默认 connector、prompt、schema、runtime Markdown 的生成和 upgrade `.example` 逻辑。
- `src/review_gate.rs` 管理 plan/code review 的 reviewer 调用、结构化 JSON 规范化、summary 和 change-doc review section 写入。
- `src/state.rs` 管理 request/session/status/approval/event/recovery 等持久化逻辑。
- `src/delivery.rs` 管理 `finish` 阶段的 git commit/push、PR body 和 PR connector。
- `src/doctor.rs` 管理环境诊断。
- `src/utils.rs` 管理时间、路径、JSON 文本解析、Markdown/TSV 转义等共享工具。
- `main.rs` 降到约 2700 行，保留命令入口、初始化、tick/agent 编排和跨模块流程。

## 关键设计点

- 这次选择 crate 内模块，而不是拆成多个 Cargo crate，避免在纯重构中引入发布结构、依赖边界和 API 设计变化。
- 模块对外只使用 `pub(crate)`，不扩大用户可见的接口面。
- 结构测试要求关键模块存在，并禁止 `main.rs` 重新出现 `deliver_finished_request`、`doctor_command_check`、`load_requests`、`run_single_reviewer`、`json_objects_in_array` 等主体实现。
- 模板和静态资产边界保持不变: 资产仍由 `src/assets.rs` 编译期 include，运行时模板仍来自 `templates/`。

## 变更范围摘要

- 新增 `src/defaults.rs`、`src/review_gate.rs`、`src/state.rs`、`src/delivery.rs`、`src/doctor.rs`、`src/utils.rs`。
- 更新 `src/main.rs` 的模块声明和 crate 内 re-export。
- 更新 `tests/cli_flow.rs` 的结构测试。
- 更新 `README.md` 和 `skills/codex-auto-dev-workflow/SKILL.md` 的源码维护结构说明。

## 验证证据

- `cargo fmt --check`: 通过。
- `cargo test templates_are_external_assets_not_embedded_in_main`: 通过。
- `cargo test`: 通过，1 个单元测试和 42 个 CLI 集成测试通过。
- `cargo check`: 通过。
- `cargo clippy --all-targets -- -D warnings`: 通过。
- `python3 scripts/validate_proposals.py`: 通过，校验 34 个 proposal。
- `git diff --check`: 通过。

## Review 结果

本次是行为保持型源码结构重构，没有运行自动 reviewer gate；以结构测试、全量 CLI 集成测试、clippy、proposal 校验和人工检查作为交付前验证。
