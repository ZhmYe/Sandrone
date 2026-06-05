# 计划: Agent Exit Advance Hook

## 目标与依赖顺序

1. 新增失败测试。
   先写集成测试，只调用一次 `tick`，期望 hook 自动推进到 implementation 和 `waiting-finish`。在没有 hook 时测试应失败。

2. 新增 `advance` 命令。
   解析 request ID 和 max attempts，只推进单个 request，不运行 update。

3. 抽出单 request 推进逻辑。
   复用 tick 的状态刷新、review gate、start 和 agent 派发逻辑，保证 tick 与 hook 行为一致。

4. 添加 per-request lock。
   使用 `.sandrone/state/locks/<request_id>.lock/` 避免 heartbeat 与 hook 并发推进。

5. 接入 agent wrapper hook。
   wrapper 写 exit code 后调用 `sandrone advance --request_id <REQ>`，输出写入 hook log。

6. 更新文档和 skill。
   说明 heartbeat 负责发现新需求和兜底恢复，hook/advance 负责即时推进。

## 测试策略

- 单测/集成测试覆盖 hook 自动推进。
- 原有 tick 刷新测试调整为验证 hook 后 tick 仍是 no pending。
- 完整运行格式、编译、clippy、测试、proposal 校验和 diff 检查。

## 风险与回滚

- 风险: hook 调用 advance 时 reviewer 较慢，hook log 会记录耗时和输出。heartbeat 仍可兜底恢复。
- 风险: lock 残留会导致 advance 跳过。当前 lock 写入 pid，发现 pid 不存在时会清理 stale lock。
- 回滚: 可以移除 wrapper hook，只保留 `advance` 手动命令和 heartbeat 兜底。
