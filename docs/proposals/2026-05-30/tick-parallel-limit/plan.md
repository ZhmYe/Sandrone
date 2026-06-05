# Plan: Tick Parallel Limit

## 目标与顺序

1. 先修改集成测试，证明默认并发从“全部派发”收敛为 1，并且 running request 会占用槽位。
2. 增加 `--parallel-limit <N>` 覆盖测试，保留需要并行时一次派发多个 request 的能力。
3. 更新配置模型，给新 workspace 写入 `parallel_limit = 1`，旧配置读取时默认 1。
4. 在 `tick` 主控里统计 running request，按剩余槽位截断待派发 request。
5. 更新 help、README、workflow skill 和 proposal 索引。
6. 运行完整验证。

## 实现位置

- `src/main.rs`: `Config`、config read/write、`tick` 参数解析和并发槽位计算。
- `tests/cli_flow.rs`: 默认并发、运行中占槽、flag 覆盖、非法参数测试。
- `README.md`、`skills/sandrone/SKILL.md`: 使用说明。
- `docs/proposals/2026-05-30/tick-parallel-limit/`: 本次变更记录。

## 设计说明

并发限制放在 `tick` 中，因为 heartbeat 是批量扫描和派发的入口。`advance` 保持单 request 推进器语义，适合 hook 在某个 request 的 agent 退出后继续推进同一个 request。这样不会把全局排队逻辑混入 per-request recovery。

并发计数使用 request status，而不是进程列表。状态文件是框架已有的调度事实源，`refresh_tick_statuses` 会先清理已结束或 stale 的 running 状态，再进入并发判断。

## 测试策略

- 用一个会短暂 sleep 的 issue-agent 保持 `planning-agent-running`，验证第二次 tick 在默认并发 1 下不会派发第二个 issue。
- 用 `--parallel-limit 2` 验证用户显式调高并发时可以同时派发两个 request。
- 用非法参数测试确保失败路径有明确错误文本。
