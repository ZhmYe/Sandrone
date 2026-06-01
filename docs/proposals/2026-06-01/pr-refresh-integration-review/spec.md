# PR Refresh Integration Review Spec

## 背景

`finish` 创建或复用 PR 后，base/master 可能继续前进，导致 request 分支过期或产生冲突。这个阶段不应重新进入普通 implementation，也不应默认运行完整 code-review；它需要一个专门的 rebase 集成支线，确认两边修改都被妥善保留。

## 目标

- 新增 `pr-refresh`，对已有 request 分支执行 fetch + rebase。
- rebase 无冲突时运行轻量 IntegrationReviewer，确认集成安全后回到 `wait-update-pr`。
- rebase 有冲突时派发 RebaseAgent，解决后仍必须运行 IntegrationReviewer。
- IntegrationReviewer 必须审查冲突标记、原需求语义、base/master 新代码保留、测试证据和 change-doc 记录。
- `finish` 支持 PR 已存在后的刷新交付: 没有新文件改动时也能重新生成 PR body 并 push，必要时使用 `--force-with-lease`。

## 非目标

- 不自动 merge PR。
- 不把 IntegrationReviewer 替代首次实现后的 TestReviewer + DesignReviewer。
- 不让 implementation agent 处理 PR rebase 冲突。

## 成功标准

- 默认 workspace 生成 `tools/rebase-agent.sh`、`tools/pr-status.sh`、`tools/integration-review.sh` 和对应 prompt/example。
- RebaseAgent prompt 明确禁止为了 request 分支删除 base/master 新代码。
- IntegrationReviewer prompt 明确检查 base/master 修改是否保留。
- 冲突 rebase 可以派发 RebaseAgent，agent 完成后自动运行 IntegrationReviewer。
- proposal 校验、Rust 测试、clippy 和 diff 检查通过。
