# Source Template Module Refactor Spec

## 背景

`src/main.rs` 已经承载 CLI 状态机、全局 registry、dashboard、默认脚本、默认 prompt、schema 和 HTML。长文本嵌入 Rust 代码后，review diff 难读，prompt/HTML 修改也容易误伤业务逻辑。

## 需求

- 将 dashboard HTML、agent/reviewer prompt、connector shell script、review schema 和 runtime Markdown 模板移出 Rust 源码，放入仓库内可直接阅读和编辑的资产目录。
- 固定不可替换的 dashboard 静态页面放入 `assets/`；会生成到 workspace 或作为默认可替换内容的文件放入 `templates/`。
- 保持默认 workspace 生成行为不变，`new`、`upgrade`、`.example` 文件和 review schema 仍使用同一套默认内容。
- 拆出 dashboard 和 registry 的 Rust 模块，降低 `main.rs` 体量。
- 增加测试约束，防止未来把长 HTML/prompt 再次写回 `main.rs`。
- 更新 README 和 Skill，说明新的源码维护边界。

## 非目标

- 不改变 tick/advance 状态机。
- 不改变 registry JSON 格式。
- 不改变 dashboard API schema。
- 不改变 reviewer 输出 schema。
- 不引入运行时读取模板文件；模板仍通过 `include_str!` 编译进二进制，避免安装后缺文件。
