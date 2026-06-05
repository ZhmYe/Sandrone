# Spec: Tick Parallel Limit

## 背景

自动 heartbeat 会周期性运行 `sandrone tick`。如果一次 tick 对所有 eligible issue 都派发 agent，多 issue 场景下会同时打开过多 Codex 子运行，容易争抢 CPU、网络、reviewer backend 和用户注意力，也会让未来前端难以清晰展示排队状态。主控层需要提供明确的并发上限。

## 目标

- 新 workspace 默认同一时间最多自动处理 1 个 issue。
- `tick` 支持 `--parallel-limit <N>` 单次覆盖并发上限。
- `.sandrone/config.toml` 支持持久配置 `parallel_limit = 1`。
- running 状态的 request 必须占用并发槽，包括 `planning-agent-running`、`implementation-agent-running` 和 legacy `agent-running`。
- 超过并发上限时，pending request 保持原状态，等待后续 tick。

## 非目标

- 不改变 `advance --request_id` 的单 request 语义；hook 继续推进同一个 request 的下一 phase。
- 不引入复杂队列、优先级或调度策略。
- 不中止已经运行的 agent。

## 行为要求

- `tick` 先运行 update 和状态刷新，再统计仍在 running 的 request。
- 如果 running 数量已经达到并发上限，`tick` 不派发新的 issue-agent，并输出明确提示。
- 如果还有剩余槽位，`tick` 只派发最多剩余槽位数量的 request。
- `--parallel-limit 0` 或非数字必须失败，并输出可匹配的错误信息，不得 panic。
- 旧 workspace 缺少 `parallel_limit` 时按默认值 1 处理；upgrade 后写回配置。

## 验证

- 默认 tick 面对两个 pending issue 只派发一个。
- 当已有一个 request 处于 running 时，默认 tick 不派发第二个 request。
- `tick --parallel-limit 2` 可以一次派发两个 request。
- `tick --parallel-limit 0` 返回明确错误。
