# README Dashboard Command Docs Change Doc

## 摘要

本次只更新 README 文档，补齐 dashboard 页面能力和相关命令说明。现在 README 明确说明 `cad` 别名、dashboard registry 数据来源、页面为空时的刷新方式、三类项目标签语义、stage 展示和局部滚动行为。

## 实现前后对比

变更前:

- README 提到 dashboard，但没有完整解释页面为空的常见原因。
- 左侧项目标签、`pending`/`finish` 语义和局部滚动行为未同步到文档。
- dashboard 命令参考没有明确 host/port 示例和旧 workspace 登记方式。

变更后:

- 安装章节说明 `cad` 可替代 `codex-auto-dev`。
- 快速开始说明页面为空时进入 workspace 运行 `cad list`、`cad upgrade` 或 `cad dashboard --json`。
- Dashboard 命令参考补充 host/port、`--port 0`、`--json`、HTTP 端点、registry 刷新和页面支持能力。
- Dashboard 展示规则说明 `blocked`、`pending`、`finish` 三类标签，其中 `pending` 包含 `waiting-finish`。

## 验证证据

- `python3 scripts/validate_proposals.py`: 通过。
- `git diff --check`: 通过。

## Review 结果

本次是 README 文档更新，没有运行自动 reviewer gate；以 proposal 校验和 diff 空白校验作为交付验证。
