# Spec: Reviewer Isolation And Runtime List Sync

## 背景

自动 code-review 有两个需要收紧的问题。第一，TestReviewer 和 DesignReviewer 都是独立门禁，但默认 reviewer 环境会暴露原始 change 目录；当 TestReviewer 写出 detail 后，DesignReviewer 理论上可以读取它，历史 review summary/detail 也可能影响当前轮判断。第二，`status.json` 已经进入 `waiting-finish` 时，`list` 和 `status` 仍可能直接读取滞后的 `.sandrone/state/requests.tsv`，导致用户看到 `implementation-agent-running`。

## 目标

- code-review 中每个 reviewer 必须拿到独立 review context。
- review context 不包含 `reviews/`、summary/detail 或 agent journal。
- TestReviewer 和 DesignReviewer 不得读取其他 reviewer 输出或历史 review 轮次。
- DesignReviewer 不得依赖 TestReviewer 的通过或拒绝结论。
- `list` 和 `status` 读取前必须先从 runtime `status.json` 同步更靠后的状态。

## 非目标

- 不阻止恶意自定义 reviewer 通过绝对路径主动读取 workspace；框架负责移除默认直接指针、声明 forbidden paths，并让默认 prompt 严格禁止。
- 不改变 reviewer schema。
- 不改变 `advance`、`tick` 的 per-request 推进语义。

## 行为要求

- reviewer 环境中的 `SANDRONE_CHANGE_PATH` 指向隔离 context，而不是原始 change 目录。
- reviewer 环境提供 `SANDRONE_REVIEW_CONTEXT` 和 `SANDRONE_REVIEW_FORBIDDEN_PATHS`。
- 隔离 context 只复制 `request.md`、`plan.md`、`change-doc.md`、`status.json` 和 `approvals/`。
- 原始 review detail 和 summary 仍写入 canonical `docs/changes/<name>/reviews/<stage>/...`。
- `list` 和 `status` 必须同步 `status.json` 中更靠后的状态、branch 和 worktree 后再输出。

## 验证

- 构造历史 code-review detail 和 summary，确认 TestReviewer 和 DesignReviewer 看到的 `SANDRONE_CHANGE_PATH` 中没有 `reviews/`。
- 确认新 review detail 仍写入 canonical review 目录。
- 构造 `requests.tsv=implementation-agent-running`、`status.json=waiting-finish`，确认 `status REQ` 和 `list` 都显示 `waiting-finish`，且中央索引被同步。
