# 规格: Clone CodeGraph And Finish Delivery

## 背景

初始化流程需要收敛到真实远端仓库: 用户先创建 GitHub 或公司 Git 仓库，再由框架 clone 到 `dev/repo`。计划前还需要判断本地代码是否落后 upstream，以及是否需要生成或刷新 CodeGraph。完成阶段也需要从“只标记 finished”升级为审批后提交、推送和准备 PR。

## 用户目标

用户希望所有开发需求都在独立 worktree 和独立分支中完成，并最终准备合并到 `master`。`finish` 在 change-doc 审批后应提交代码、push 到 request 分支，并通过可替换脚本创建 PR；如果无法创建 PR，应给出手动 PR 链接或失败原因。

## 功能要求

- 推荐初始化方式统一为 `codex-auto-dev new --url <repo-url>`。
- 用户应先创建远端仓库；`new --name` 仅作为本地兼容/测试入口保留。
- `new --url` clone 后判断仓库是否为空。
- 空仓库跳过 CodeGraph，等待用户给需求。
- 有内容的仓库提示 Codex 在计划前运行 `codegraph-project-preview`。
- `plan` 前执行同步检查。如果本地分支落后 upstream，必须失败并提示先 `git pull`。
- `plan` 前判断是否需要生成或刷新 CodeGraph，并把结论写入计划模板和 handoff。
- `start` 仍然创建 request 独立 worktree 和 `codex/<request_id>` 分支。
- `finish --message <commit-message>` 在 change-doc approval 通过后:
  - 在 request worktree 中 `git add -A`。
  - 用规范 commit message 提交。
  - push 到 request 分支。
  - 调用 `tools/pr-create.sh` 尝试创建到 `master` 的 PR。
  - PR title 使用 commit message。
  - PR body 使用关联需求信息 + `change-doc.md` + `tasks.md`。
  - 默认 GitHub issue 的 `external_id` 形如 `github:owner/repo#42` 时，PR body 写入 `Closes owner/repo#42` 来关联 issue。
  - `tools/pr-create.sh` 和 `tools/issue-update.sh` 一样可替换，支持 GitHub 以外的平台。
  - PR 创建失败时输出手动 PR 链接或失败原因。
  - 标记 request finished。
- `finish` 不得 merge。

## 非目标

- 不自动创建远端仓库。
- 不自动 merge PR。
- 不在 implementation thread 中执行 commit/push/PR；交付动作集中在 `finish`。
- 不强制运行 CodeGraph，只判断并记录是否需要运行；Codex 根据 skill 执行。

## 验收标准

- clone 空仓库时输出跳过 CodeGraph。
- clone 有内容仓库时输出 CodeGraph required。
- 远端有新提交时，`plan` 失败且不创建 change packet。
- `finish` 未通过 change-doc approval 时失败。
- `finish` 通过审批后能 commit、push 到 `codex/req-0001`。
- `finish` 通过 PR connector 输出 PR 创建结果或手动 PR 信息。
- PR body 包含 request/issue 关联信息；GitHub issue 能通过 closing keyword 自动关联。
