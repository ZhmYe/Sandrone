# Self Workspace Guard Spec

## 背景

框架源码仓库根目录曾被误当作 managed workspace 初始化，生成了 `.codex-auto-dev/`、`dev/` 和 `tools/` 运行态目录。虽然这些目录已被 `.gitignore` 忽略，不会进入提交，但会污染源码 checkout，并可能让 dashboard/list 把框架自身当作目标项目。

## 需求

- 清理当前源码仓库根目录中误生成的运行态目录。
- `codex-auto-dev new` 必须拒绝在 `codex-auto-dev-workflow` 源码 checkout 根目录执行。
- 拒绝时不能创建 `.codex-auto-dev/`、`dev/` 或 `tools/`。
- README 和 Skill 必须说明不要在框架源码仓库里初始化 managed workspace。

## 非目标

- 不影响普通目标项目目录中的 `codex-auto-dev new`。
- 不删除构建缓存 `target/`、CodeGraph 缓存或其他开发工具缓存。
- 不改变已存在 managed workspace 的升级流程。
