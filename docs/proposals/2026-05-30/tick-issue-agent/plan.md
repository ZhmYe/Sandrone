# 计划: Tick Issue Agent

## 目标依赖图

1. 简化 runtime 文档包。
   先把 `plan` 生成物从 spec-kit 风格改为必要文档，避免后续 tick 和 issue-agent 依赖旧文件。
2. Issue agent assets。
   依赖文档包稳定，新增默认 issue-agent connector 和 prompt。
3. Tick 派发。
   依赖 issue agent assets，新增 `tick` 短主控: update、刷新已结束 agent 状态、为全部 eligible request 生成 change packet、异步派发 issue-agent。
4. 阻塞与恢复。
   依赖状态文件，新增 `block` 和 `resume`，让超过最大轮数后能快速接手。
5. Review 汇总。
   依赖 review 结果，写 details/summary，并把最终结论同步到 `change-doc.md`。

## 代码改动

- 修改 `src/main.rs`:
  - 新增 `ISSUE_AGENT_TOOL` 和 `ISSUE_AGENT_PROMPT`。
  - 新增 `tick`、`block`、`resume` 命令。
  - 修改 `generate_plan_packet`，生成 `request.md`、`plan.md`、`change-doc.md`、`agent-journal.md`、`status.json` 和 approvals。
  - 修改 review 输出路径为 `reviews/<stage>/details/<attempt>-<reviewer>.json`。
  - 修改 `write_pr_body`，不再依赖 `tasks.md`。
  - 初始化和 upgrade 时补齐 issue-agent assets。
  - `validate` 改为校验简化文档包。
- 修改 `tests/cli_flow.rs`:
  - 更新旧模板断言。
  - 新增 issue-agent asset 测试。
  - 新增 tick 批量异步派发测试和后续状态刷新测试。
  - 新增 block/resume 测试。
  - 新增 review summary 进入 change-doc 测试。
- 修改 README 和 skill:
  - 说明 heartbeat/tick 只派发，issue-agent 负责连续上下文和 review 修复循环。

## 测试策略

- 使用 shell 脚本模拟 issue source、issue-agent、reviewers 和 PR connector。
- 失败路径必须匹配明确错误文本。
- targeted tests 先覆盖新增命令，再跑完整门禁。

## 风险与回滚

- 简化 runtime 文档会影响旧 tests 和旧 workspace；`upgrade` 需要补齐新文档，但保留已填写旧文档。
- issue-agent 默认依赖 Codex CLI；可替换脚本保证未来可接入其他 agent。
- 回滚时可以继续使用手动 `plan/start/review/finish` 流程。
- 并发 issue-agent 会共享框架状态文件；第一版通过短 CLI 命令和 agent 状态文件降低主控阻塞，后续可以补更强的状态锁。
