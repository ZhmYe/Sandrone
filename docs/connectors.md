# Connector 契约

`tools/*.sh` 都是可替换 connector。默认实现使用 GitHub、Codex CLI 和 Rust 检查；内部平台可以替换脚本，只要输入输出稳定。

## `tools/issue-update.sh`

stdout 输出零行或多行 TSV，无 header：

```text
external_id<TAB>source<TAB>title<TAB>body<TAB>url
```

字段要求：

- `external_id`：稳定且全局可去重，例如 `github:owner/repo#123`。
- `source`：短平台名，例如 `github`、`jira`、`internal`。
- `title`：规范化需求名称。
- `body`：完整需求描述，不能只给标题。
- `url`：原始需求链接，可为空。

`sdr update` 会按 `external_id` 去重。已存在 request 会刷新标题、描述、URL 和更新时间，不会重复创建新 request。

## `tools/issue-agent.sh`

这是 agent 后端 connector，不是单体业务提示词。默认实现会组合：

- `tools/prompts/issue-agent.md`：共享 agent 契约。
- `tools/prompts/decomposition-agent.md`：拆解阶段提示词。
- `tools/prompts/plan-agent.md`：计划阶段提示词。
- `tools/prompts/implementation-agent.md`：实现阶段提示词。
- `tools/prompts/rebase-agent.md`：PR refresh 冲突阶段提示词。

常见 phase：

| Phase | 允许做什么 | 不允许做什么 |
| --- | --- | --- |
| `decomposition` | 写 `<REQ> decomposition.md`、`decomposition.json`、`dag.json`、Obsidian 导航和 agent journal。 | 改目标代码、跑 review、approve、commit、push。 |
| `planning` | 写 `<REQ-SNN> plan.md` 和 agent journal。 | 改目标代码、跑 review、start、commit、push。 |
| `implementation` | 在 `dev/worktrees/<REQ-SNN>` 写代码，更新 `<REQ-SNN> change-doc.md` 和 agent journal。 | 改 `dev/repo`、跑 reviewer gate、approve、commit、push。 |
| `rebase` | 解决 rebase/集成冲突，保留 base/master 新代码和已通过实现语义。 | 扩大需求、commit、push、finish、merge。 |

agent 必须在退出前自检 reviewer 会检查的内容。明显会产生 critical/high 的问题，应先修复或 block，不要浪费 review 轮次。

## `tools/check-format.sh`

code-review 前置检查 connector，支持：

```bash
tools/check-format.sh --format
tools/check-format.sh --check
```

默认 Rust 实现：

- `--format`：运行 `cargo fmt`。
- `--check`：运行 `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets --all-features -- -D warnings`。
- 非 Rust 项目默认明确 skip。

`code-review` 会先运行 `--check`。失败时不会调用 TestReviewer/DesignReviewer，而是写入 `checks/format-check.md`，把 request/slice 回退到 implementation。

## Reviewer 脚本

默认 reviewer：

- `tools/decomposition-review.sh`
- `tools/plan-review.sh`
- `tools/test-review.sh`
- `tools/design-review.sh`
- `tools/integration-review.sh`

stdout 必须是符合 `tools/schemas/review-result.schema.json` 的 JSON 对象。非法 JSON、空输出、schema 不匹配或脚本失败都会成为 blocking review。

最小结构示例：

```json
{
  "reviewer": "PlanReviewer",
  "approved": false,
  "gate_unavailable": false,
  "decision": "rejected",
  "recommended_next_phase": "planning",
  "summary": "计划缺少关键测试设计。",
  "process": ["读取 request", "读取 plan", "检查风险和测试"],
  "critical": [],
  "high": [
    {
      "title": "缺少失败路径测试",
      "evidence": "plan 只列出 happy path。",
      "impact": "实现可能无法覆盖错误输入。",
      "required_fix": "补充错误输入、边界条件和回归测试计划。",
      "suggested_change": "在测试章节列出具体测试文件和断言。",
      "verification": "reviewer 重新检查测试章节。"
    }
  ],
  "warning": [],
  "info": []
}
```

规则：

- 任意 `critical` 或 `high` 非空，`approved` 必须是 `false`。
- `gate_unavailable=true` 必须 block，不能绕过。
- finding 必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change`、`verification`。
- reviewer 必须返回 `recommended_next_phase`：`planning`、`implementation` 或 `blocked`。
- TestReviewer 与 DesignReviewer 必须独立评审，不读取对方输出、历史 review detail、summary 或 agent journal。

## PR 脚本

### `tools/pr-create.sh`

finish 时调用。脚本必须先判断平台是否支持创建 PR，再检查 base/head 是否已有 PR。

成功 stdout：

```text
created<TAB>url
```

或：

```text
existing<TAB>url
```

失败时 stderr 输出原因，不得 merge。

### `tools/pr-status.sh`

只观察 PR 状态，不修改代码、分支或 PR。

成功 stdout：

```text
status<TAB>url<TAB>detail
```

推荐 `status`：`open`、`missing`、`merged`、`closed`、`unknown`。只有 `merged` 才能把 request 标记为 `finished`。

## PR Body

`finish` 生成的 PR body 应包含：

- request 来源和链接。
- plan/change-doc/task 进度引用。
- change-doc 摘要。
- 自动评审意见，尤其是 warning/info finding 的证据、影响、必要修复、建议修改和验证方式。

这样人类 reviewer 可以在 PR 平台直接理解风险，不必回本地翻 JSON。
