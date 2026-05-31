# Plan: Worktree Start Auto Pull

## 实施步骤

1. 增加 start 回归测试: plan approval 后远端新增提交，start 必须自动 pull 并基于最新代码创建 worktree。
2. 增加失败路径测试: 本地和远端分叉时，start 必须 block 且不得创建 worktree。
3. 在 `start_worktree` 中，在创建新 worktree 前执行 `git pull --ff-only`。
4. pull 失败时复用 `mark_blocked` 写入 `requests.tsv`、`status.json` 和 `recovery.md`。
5. 更新 README、workflow skill、proposal 索引。
6. 运行格式、编译、clippy、测试、proposal 校验和 diff 检查。

## 改动位置

- `src/main.rs`: worktree 创建前的目标仓库同步逻辑。
- `tests/cli_flow.rs`: start 自动 pull 和 pull 失败 block 测试。
- `README.md`: worktree 创建前同步规则。
- `skills/codex-auto-dev-workflow/SKILL.md`: skill 中的 start/worktree 门禁说明。
- `docs/proposals/2026-05-31/worktree-start-auto-pull/`: 本次框架变更记录。

## 风险与兼容

- 使用 `--ff-only`，不会自动 merge 或 rebase。
- 只在新建 worktree 前同步，不影响已经存在的 request worktree。
- 无 remote 或空仓库继续跳过 pull。
