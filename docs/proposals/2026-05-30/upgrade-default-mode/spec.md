# Spec: Upgrade Default Mode

## 背景

`upgrade` 需要同时满足两个相反需求: 默认保护用户定制的 connector、prompt 和 review schema；在用户确认没有本地定制时，也要有一个明确命令把最新版默认实现安装回正式文件。

## 目标

- `new` 生成正式默认文件和同内容 `.example.*` 文件。
- 普通 `upgrade` 只刷新 `.example.*` 参考文件，不创建或覆盖正式 connector、prompt、review schema。
- 普通 `upgrade` 输出提醒用户自行决定复制哪些 example。
- 新增 `upgrade --default`，先刷新 `.example.*`，再用 example 覆盖对应正式文件。
- `upgrade --dry-run --default` 输出将被替换的正式文件和来源 example。

## 非目标

- 不自动判断用户脚本是否“定制过”。
- 不做三方 merge、diff UI 或交互式选择。
- 不改变 request 状态机、review gate 或 finish 逻辑。

## 行为要求

- 没有 `--default` 时，正式运行资产完全由用户控制。
- `--default` 是显式破坏性替换正式运行资产的确认信号，但只影响框架默认管理的 connector、prompt 和 schema。
- `.example.*` 文件始终由框架维护，可在 upgrade 中刷新。
- 输出必须能让用户看懂普通 upgrade 不会替换正式脚本，以及下一步可以人工复制或运行 `--default`。

## 验证

- 集成测试确认 `new` 的正式文件和 example 内容一致。
- 集成测试确认普通 `upgrade` 不覆盖也不补正式脚本，只刷新 example 并输出提醒。
- 集成测试确认 `upgrade --default` 用 example 覆盖正式脚本、prompt 和 schema。
