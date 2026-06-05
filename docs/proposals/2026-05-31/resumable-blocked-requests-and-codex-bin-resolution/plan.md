# Plan: Resumable Blocked Requests And Codex Bin Resolution

## 实施步骤

1. 增加默认脚本生成测试，要求 agent/reviewer connector 支持 `SANDRONE_CODEX_BIN` 和 `SANDRONE_CODEX_APP`，且不写死 app 路径。
2. 增加 resume 回归测试，证明 blocked request 被写回可派发状态并能被 `tick --request_id` 派发。
3. 在默认 agent/reviewer scripts 中加入 `resolve_codex_bin`，按 env、PATH、app bundle 顺序解析。
4. 修改 `resume`，对 blocked request 更新 `requests.tsv`、`status.json`、session 和事件流。
5. 更新 README、workflow skill、proposal 索引。
6. 运行格式、编译、clippy、测试、proposal 校验和 diff 检查。

## 改动位置

- `src/main.rs`: 默认 connector 生成、resume 状态恢复。
- `tests/cli_flow.rs`: 默认脚本生成测试和 resume 派发测试。
- `README.md`: connector 配置与 resume 行为说明。
- `skills/sandrone/SKILL.md`: skill 中的默认 connector 和恢复契约。
- `docs/proposals/2026-05-31/resumable-blocked-requests-and-codex-bin-resolution/`: 本次变更记录。

## 风险与兼容

- 保持 PATH 中 `codex` 的旧行为。
- 新增环境变量是可选配置，不影响自定义 connector。
- resume 只对 `blocked` request 改状态；非 blocked request 仍只打印恢复信息。
