# Spec: PR Create Capability And Existing Detection

## 背景

`finish` 会在 commit/push 后调用 `tools/pr-create.sh`。旧默认脚本直接执行 `gh pr create`，没有先判断当前平台是否支持 PR，也没有检查同一个 base/head 是否已经创建过 PR。对于 GitHub 以外的平台或重复执行 finish/recovery 场景，这会导致错误信息不清晰，甚至重复尝试创建 PR。

## 目标

- PR connector 必须先判断当前平台/仓库是否支持创建 PR。
- PR connector 必须在创建前检查 base/head 是否已经存在 PR。
- connector 成功输出必须能区分新建 PR 和已有 PR。
- `finish` 必须识别已有 PR，并向用户报告 `PR already exists`。
- 保持兼容旧 connector: stdout 只输出 URL 时仍按新建 PR 处理。

## 非目标

- 不把 PR 创建逻辑写死在 Rust 代码里。
- 不只支持 GitHub；默认脚本使用 GitHub CLI，但 contract 要允许 GitLab、Gerrit、Bitbucket 或内部平台替换。
- 不 merge PR。

## 行为要求

- `tools/pr-create.sh` 成功时 stdout 输出一个 TSV 行: `created<TAB>url` 或 `existing<TAB>url`。
- 如果平台不可用、鉴权不可用、仓库不是可创建 PR 的平台，或无法安全检查已有 PR，脚本必须非 0 退出并写 stderr。
- 默认 GitHub 脚本必须先运行可访问性检查，再用 `gh pr list --state all --base ... --head ...` 检查已有 PR，最后才运行 `gh pr create`。
- Rust `finish` 解析 `existing<TAB>url` 时输出 `PR already exists: <url>`。
- Rust `finish` 解析旧裸 URL 时输出 `PR created: <url>`。

## 验证

- 新 workspace 生成的默认 `tools/pr-create.sh` 包含 `created<TAB>url`、`existing<TAB>url`、`gh pr list` 和 `gh pr create`。
- 自定义 connector 输出 `existing<TAB>url` 时，`finish` 报告已有 PR 而不是新建 PR。
- 既有裸 URL connector 仍能通过旧测试。
