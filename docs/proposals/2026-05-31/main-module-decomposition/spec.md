# Main Module Decomposition Spec

## 背景

`src/main.rs` 已经承载 CLI 分发、workspace 初始化、tick 状态机、review gate、PR 交付、默认模板生成、状态持久化和通用工具函数。文件超过 5000 行后，后续维护 dashboard、agent 编排、review 门禁时很难快速定位问题，也容易在无关逻辑之间产生误改。

## 需求

- 在不改变业务流程和命令行为的前提下，将 `main.rs` 中边界清晰的主体逻辑拆到 crate 内模块。
- 保持二进制入口、命令名称、参数、状态文件、生成文件路径和测试行为不变。
- 给模块边界增加测试约束，避免后续把长模板、review 逻辑、状态持久化重新塞回 `main.rs`。
- 更新 README 和 Skill 的源码维护结构说明。

## 非目标

- 不把项目改成 Cargo workspace 或多 crate 发布结构。
- 不重写状态机、不调整 agent 派发策略、不改变 review schema。
- 不引入新的第三方依赖。
