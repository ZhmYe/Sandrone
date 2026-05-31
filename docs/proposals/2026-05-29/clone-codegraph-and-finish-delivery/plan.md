# 计划: Clone CodeGraph And Finish Delivery

## 目标依赖图

1. 统一 clone 初始化语义。
   先完成仓库内容检测，才能提示 CodeGraph。
2. 计划前检查。
   依赖 clone 后的目标仓库状态，先判断 pull，再判断 CodeGraph。
3. finish 交付动作。
   依赖已有 approval、worktree 和 request branch。
4. 文档与 skill 更新。
   依赖 CLI 语义稳定。

## 代码改动

- 修改 `src/main.rs`:
  - `new --url` 默认 base branch 改为 `master`。
  - clone 后用 `rev-parse HEAD` 判断仓库是否为空。
  - `plan` 前 fetch upstream，并用 `rev-list HEAD..@{u}` 判断是否需要 pull。
  - `plan` 前根据 `docs/codegraph/context.md` 是否存在、是否早于最新 commit 判断是否需要 CodeGraph。
  - 将 preflight 结论写入 `plan.md`、`codex-plan.md` 和 `thread-handoff.md`。
  - `finish` 支持 `--message`。
  - `finish` 在 worktree 中 commit、push 分支，并调用 `tools/pr-create.sh`。
  - `finish` 使用关联需求信息 + `change-doc.md` + `tasks.md` 生成 PR body。
  - 默认 GitHub issue 写入 `Closes owner/repo#42` 关联 issue；其他平台由 PR connector 自行处理。
  - PR connector 创建失败时输出 compare 链接或失败原因。
- 修改 `tests/cli_flow.rs`:
  - 覆盖空仓库跳过 CodeGraph。
  - 覆盖有内容仓库要求 CodeGraph。
  - 覆盖远端更新导致 plan 被阻止。
  - 覆盖 finish commit/push/request branch。
  - 覆盖 PR connector 调用、PR body issue 关联和 URL 输出。
- 修改 `README.md` 和 `skills/codex-auto-dev-workflow/SKILL.md`:
  - 推荐用户先创建远端仓库再 clone。
  - 说明 plan 前 pull/CodeGraph 检查。
  - 说明 finish 交付到 request 分支并准备 PR。
- 修改 `.specify/memory/constitution.md`:
  - 允许 change-doc 审批后的 finish-time delivery。

## 测试策略

- 使用本地 bare git 仓库模拟远端。
- 使用第二个 clone 推送新提交，验证 plan 发现本地落后。
- 在 request worktree 中写入文件，验证 finish 产生 commit 并 push 到 bare remote 的 `codex/req-0001`。
- 全量运行格式化、检查、clippy、集成测试和 proposal 校验。

## 风险与回滚

- 默认 `tools/pr-create.sh` 依赖 GitHub CLI 和认证。失败时不阻断 finish，会输出手动 PR 信息。
- 非 GitHub remote 需要替换 `tools/pr-create.sh`，否则只能输出失败原因。
- 如果 worktree 没有 git user 配置，commit 会失败并保留错误信息。
