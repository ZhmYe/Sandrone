# Assets Template Boundary Spec

## 背景

`templates/` 应该表达“会被填充、复制或替换的模板”。dashboard HTML 是框架内置的静态页面，编译进二进制后由 dashboard HTTP 服务直接返回，不会被复制到 managed workspace，也不是用户需要替换的默认 connector/prompt。

## 需求

- 将 dashboard HTML 从 `templates/dashboard/index.html` 移到 `assets/dashboard/index.html`。
- 保持 `src/assets.rs` 作为编译期入口，但 include 路径要指向新的 `assets/` 位置。
- 更新测试、README、Skill 和相关 proposal 文档，明确 `assets/` 与 `templates/` 的边界。
- 保持 dashboard 行为和 API 不变。

## 非目标

- 不移动 prompt、connector script、runtime Markdown 或 schema；这些仍属于可生成或可替换模板。
- 不改变 dashboard UI。
- 不改变 workspace 生成内容。
