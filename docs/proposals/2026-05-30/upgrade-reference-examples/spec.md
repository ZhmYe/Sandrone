# Spec: Upgrade Reference Examples

## 背景

旧 workspace 里用户可能已经改过 `tools/*.sh`、`tools/prompts/*.md` 或 review schema。`upgrade` 不能覆盖这些正式运行文件，否则会破坏本地平台接入和审核策略。但框架默认 connector、prompt、schema 会持续演进，旧 workspace 仍需要一种安全方式拿到新版参考实现。

## 目标

- `upgrade` 继续保护用户正式 connector、prompt 和 review schema。
- `upgrade --dry-run` 明确显示将刷新哪些 `.example.*` 参考文件。
- `upgrade` 每次刷新框架维护的脚本、prompt 和 schema example。
- 新建 workspace 也生成同一套 example，便于测试和对比。
- 文档说明 `.example.*` 是参考文件，正式运行仍使用无 `.example` 后缀的文件。

## 非目标

- 不自动把 example 覆盖到正式 connector。
- 不推断用户脚本是否“过旧”。
- 不新增交互式 merge 或 diff UI。
- 不改变 agent、reviewer、finish 的状态机。

## 行为要求

- `tools/issue-update.sh` 等正式脚本存在时必须保留原内容。
- `tools/issue-update.example.sh` 等 example 文件由框架维护，可以被 upgrade 刷新。
- example 脚本应与默认正式脚本共享同一份默认内容，避免模板漂移。
- review schema example 必须包含当前严格字段，例如 `gate_unavailable` 和 `recommended_next_phase`。
- 失败路径应有测试，确认 dry-run 输出、正式文件保护和 example 刷新都生效。

## 验证

- 更新 upgrade 集成测试，先写入自定义 `tools/issue-update.sh`，再确认 upgrade 不覆盖它。
- 同一测试确认 dry-run 输出 `Would refresh ...example...`。
- 同一测试确认 example 脚本、prompt 和 schema 存在并包含新版关键内容。
