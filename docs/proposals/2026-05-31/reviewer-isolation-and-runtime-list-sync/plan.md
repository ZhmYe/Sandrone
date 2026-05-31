# Plan: Reviewer Isolation And Runtime List Sync

## 目标与顺序

1. 先写 reviewer 隔离测试，模拟历史 review 输出存在时 reviewer 不应看到 `reviews/`。
2. 写 `list/status` stale index 测试，复现 runtime 已 `waiting-finish` 但 TSV 仍为 running 的问题。
3. 实现 per-reviewer context，复制必要文档但排除 review 历史和 journal。
4. 更新默认 reviewer connector 和 prompt，明确独立评审边界。
5. 实现 `list/status` 入口的 runtime sync。
6. 更新 README、workflow skill、proposal 索引并运行完整验证。

## 实现位置

- `src/main.rs`: review runner、review context 创建、默认 reviewer tool/prompt、list/status 同步。
- `tests/cli_flow.rs`: reviewer 隔离和 stale list/status 回归测试。
- `README.md`、`skills/codex-auto-dev-workflow/SKILL.md`: reviewer 独立性说明。
- `docs/proposals/2026-05-31/reviewer-isolation-and-runtime-list-sync/`: 本次变更记录。

## 设计说明

reviewer 是门禁，不是协作讨论。TestReviewer 和 DesignReviewer 应基于同一份需求、approved plan、change-doc 和 worktree 独立判断。框架为每个 reviewer 生成 `.codex-auto-dev/state/review-contexts/<request>/<stage>/<attempt>/<reviewer>/`，只复制必要交付文档和 approval 文件，避免把历史 review 意见作为输入。

`list/status` 是用户和前端最常用的观察入口。它们输出前调用 runtime sync，只把 `status.json` 中更靠后的状态同步回 TSV，不回退状态。

## 测试策略

- reviewer 隔离测试用自定义 reviewer connector 检查 `CODEX_AUTO_DEV_CHANGE_PATH` 等于 `CODEX_AUTO_DEV_REVIEW_CONTEXT`，并确认该目录没有 `reviews/`。
- stale list/status 测试手动把 TSV 回退到 `implementation-agent-running`，保留 runtime `waiting-finish`，确认两个命令都会同步并输出真实状态。
- 全量测试覆盖既有 review、tick、advance、finish 行为不回归。
