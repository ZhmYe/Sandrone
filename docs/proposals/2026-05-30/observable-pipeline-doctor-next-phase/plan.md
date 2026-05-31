# Plan: Observable Pipeline Doctor And Next Phase Reviews

## 实施顺序

1. 测试先行
   - 写 `doctor_reports_workspace_and_reviewer_readiness`。
   - 写 `events_stream_records_discovery_planning_and_dispatch`。
   - 写 `code_review_can_recommend_returning_to_planning`。

2. Doctor
   - 增加 `doctor` CLI 分支。
   - 检查 workspace、命令、目标仓库、connector、schema 和 events state 目录。
   - 输出中文/英文稳定可读报告，不泄露环境变量值。

3. Events
   - 增加 `EVENTS_PATH` 和 `append_event`。
   - 在 workspace 初始化、request 发现、change packet 创建、agent 派发、gate 提交/批准、review rejected、blocked 等关键位置写事件。

4. Review next phase
   - 扩展 `ReviewResult`。
   - 扩展 schema、fallback JSON、默认 reviewer prompt 和 summary。
   - code-review rejected 时按 `recommended_next_phase` 决定回 planning、implementation 或 blocked。
   - `next_agent_phase` 支持 `plan-review-rejected` 优先回 planning，即使旧 plan approval 文件仍存在。

5. 文档与验证
   - 更新 README 和 skill。
   - 运行格式、编译、clippy、测试、proposal 校验和 diff whitespace 检查。

## 风险

- Review schema 变严格会让旧 workspace 的自定义 reviewer 需要同步更新。
- `events.ndjson` 是追加日志，不负责替代 `requests.tsv` 或 `status.json`。
- 回 planning 时会保留旧 approval 文件，但状态机优先按 `plan-review-rejected` 决定下一轮 planning；下一次提交 plan gate 会覆盖 approval 记录。
