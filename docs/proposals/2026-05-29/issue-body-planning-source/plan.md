# 计划: Issue Body Planning Source

## 目标依赖图

1. 修正默认 GitHub connector。
   先确保 issue body 能稳定进入框架。
2. 强化 runtime 模板与 handoff。
   依赖 connector 输出 body，明确 title/body 的计划语义。
3. 更新文档和 skill。
   依赖 CLI 语义稳定。

## 代码改动

- 修改 `src/main.rs`:
  - 默认 `tools/issue-update.sh` 使用 `gh api --method GET`。
  - 默认 connector 增加 `--paginate`。
  - 默认 connector 继续输出 `.title`、`(.body // "")` 和 `.html_url`。
  - `issue.md` 拆分 `需求标题` 和 `需求描述`。
  - `codex-plan.md`、`thread-handoff.md` 和 `tasks.md` 明确要求标题和描述都作为需求来源。
- 修改 `tests/cli_flow.rs`:
  - 覆盖默认 connector 的 GET、pagination 和 body 输出表达式。
  - 覆盖 plan packet 中 issue body 与 planning prompt 的要求。
- 修改 README 和 skill:
  - 说明 connector 输出 title/body。
  - 说明 plan 不得只基于标题。

## 测试策略

- 先写集成测试，让旧实现因为缺少 `需求标题`、`需求描述` 和 GET connector 要求而失败。
- 修改实现后运行目标测试。
- 最后运行格式化、检查、clippy、完整测试和 proposal 校验。

## 风险与回滚

- 只改变新生成 workspace 的默认 `tools/issue-update.sh`。旧 workspace 的自定义 connector 不会被覆盖，需要用户按需手动更新或重新生成。
- 如果 issue 描述主要存在 comments 中，本次仍然只拿 issue body，后续需要单独扩展 comments connector。
