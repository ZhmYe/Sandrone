# 变更文档: Clone CodeGraph And Finish Delivery

## 摘要

本次变更把推荐初始化方式收敛为 clone 远端仓库，并在计划前加入 git pull / CodeGraph 判断。`finish` 从单纯状态标记升级为 change-doc 审批后的交付动作: commit、push request 分支，并通过可替换 PR connector 尝试创建 PR。

## 实现前后对比

- 实现前: 从零项目可以在本地 `new --name` 创建空仓库；`new --url` 不区分空仓库和已有内容；`plan` 不判断本地是否落后远端，也不判断 CodeGraph 是否需要刷新；`finish` 只标记状态，不提交或推送。
- 实现后: 推荐用户先创建远端仓库再 `new --url` clone。clone 后空仓库跳过 CodeGraph，有内容仓库提示先 CodeGraph。`plan` 会阻止落后 upstream 的仓库继续创建计划包，并记录 CodeGraph 检查结果。`finish` 在 change-doc approval 后提交并推送 request 分支，调用 `tools/pr-create.sh` 尝试 PR 或输出 fallback，并在 PR body 中写入需求关联信息。

## 关键设计点

### 统一 Clone 初始化

框架不再推荐本地凭 `--name` 创建目标仓库。真实使用时，用户先创建 GitHub 或公司 Git 仓库，再由 CLI clone 到 `dev/repo`。这让后续 push、PR 和机器人流程都有明确远端。

### Plan Preflight

`plan` 前先 fetch upstream。如果本地 `HEAD..@{u}` 有提交，命令失败并提示先 `git pull`。这样 Codex 不会基于过期代码写计划。CodeGraph 检查根据 `docs/codegraph/context.md` 是否存在以及是否早于最新 commit 给出结论，并写入 plan 和 handoff。

### Finish-Time Delivery

实现 thread 仍然不能 commit/push。只有 `change-doc` approval 通过后，`finish` 才在 request worktree 中 `git add -A`、commit、push 到 `codex/<request_id>`。

### PR Connector 与 Issue 关联

PR 创建被抽象为 `tools/pr-create.sh`，与 `tools/issue-update.sh` 一样可替换。CLI 负责准备 PR title、PR body、base/head 分支、request metadata 和 compare URL，并通过环境变量传给 connector。默认脚本使用 `gh pr create`；内部 Git 平台可以替换脚本并复用相同输入。

PR body 现在先写入“关联需求”部分，再拼接 `change-doc.md` 和 `tasks.md`。当需求来自默认 GitHub connector，`external_id` 形如 `github:owner/repo#42` 时，CLI 会写入 `Closes owner/repo#42`，让 GitHub PR 自动关联对应 issue。

## 变更范围摘要

主要改动集中在 CLI 的 `new`、`plan`、`finish` 流程，新增 plan preflight、finish delivery helper 和 PR connector；测试使用本地 bare remote 覆盖 push 分支，并用替换后的 `tools/pr-create.sh` 验证 PR body issue 关联；文档和 skill 更新为统一 clone、CodeGraph、finish delivery 和平台连接器语义。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`

## 风险与后续

- 默认 PR connector 依赖本机 GitHub CLI 认证和网络；失败时输出 fallback，不影响已完成的 commit/push。
- 非 GitHub 或公司内部平台需要替换 `tools/pr-create.sh`，否则只能报告失败原因。
- 未来可以为常见平台补充 connector 模板和机器人审批集成。
