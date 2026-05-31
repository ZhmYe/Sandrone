# Main Module Decomposition Plan

## 目标与依赖顺序

1. 先扩展结构测试，要求关键模块文件存在，并限制 `main.rs` 的体量。
2. 抽取默认 workspace 生成和默认资产逻辑到 `src/defaults.rs`。
3. 抽取 review 门禁逻辑到 `src/review_gate.rs`。
4. 抽取状态持久化、approval、事件流逻辑到 `src/state.rs`。
5. 抽取 `finish` 交付逻辑到 `src/delivery.rs`。
6. 抽取环境诊断逻辑到 `src/doctor.rs`。
7. 抽取共享工具函数到 `src/utils.rs`。
8. 更新 README、Skill 和 proposal 索引。
9. 运行格式化、测试、clippy、proposal 校验和 diff 空白校验。

## 设计规则

- `main.rs` 保留命令分发、初始化入口、tick/agent 编排和跨模块流程胶水。
- 各模块使用 `pub(crate)` 暴露 crate 内调用所需函数，不新增对外 API。
- 函数体保持机械搬移，除必要的模块声明、可见性和格式化外不改变逻辑。
- 结构测试禁止 `main.rs` 重新包含长模板、review runner、delivery runner、状态加载器和 JSON array parser。

## 风险与验证

- 主要风险是模块可见性遗漏，使用 `cargo check`、全量测试和 clippy 捕获。
- 行为风险来自机械搬移中的漏改，使用现有 CLI 集成测试覆盖初始化、update、plan、review、tick、dashboard、upgrade、finish 等流程。
