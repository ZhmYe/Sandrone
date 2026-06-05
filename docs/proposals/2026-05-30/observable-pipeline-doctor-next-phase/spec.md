# Spec: Observable Pipeline Doctor And Next Phase Reviews

## 背景

自动流程已经可以通过 `tick`、agent wrapper hook 和 `advance` 从 issue 发现推进到 `waiting-finish`。下一步需要提升无人值守质量: 运行前可以自检环境，流程中可以留下可追溯事件，reviewer 可以明确告诉状态机下一轮应回 planning、implementation 还是 blocked。

## 目标

- 新增 `sandrone doctor`，检查 workspace、Git、Codex CLI、GitHub CLI、目标仓库、agent/reviewer connector、review schema 和事件流目录。
- 新增 `.sandrone/state/events.ndjson`，为关键状态变化追加 JSON Lines 事件。
- 扩展 review schema，要求 `recommended_next_phase` 字段。
- code-review rejected 时根据 reviewer 建议回到 planning、implementation 或 blocked。
- 更新 README、skill、默认 reviewer prompt 和测试。

## 非目标

- 不新增终端观察面板。
- 不新增前端或 HTTP server。
- 不自动运行 `finish`、commit、push 或 PR。
- 不依赖 oh-my-codex 运行时。

## 行为要求

- `doctor` 不 panic；可选工具缺失显示 warning，阻塞性 workspace 问题显示 fail。
- 每行 event 必须是独立 JSON 对象，包含 time、event、request_id、phase、status 和 detail。
- reviewer JSON 必须包含 `recommended_next_phase`，取值只能是 `planning`、`implementation` 或 `blocked`。
- `gate_unavailable=true` 必须失败并 block。
- code-review 的任一 reviewer 推荐 `planning` 时，request 回到 planning agent，而不是继续 implementation。

## 验证

- 新增 doctor 集成测试。
- 新增事件流集成测试。
- 新增 code-review 推荐回 planning 的集成测试。
- 保持现有 tick、advance、review、finish 流程测试通过。
