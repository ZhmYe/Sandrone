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

agent 成功完成当前 phase 后，最后更新 `$SANDRONE_AGENT_STATUS_DOC` 的 YAML frontmatter。状态头必须包含 `request_id`、当前 `agent_phase`、`agent_status: submitted` 和 `agent_ready_for_review: true`。implementation/rebase 还应写入简洁的 `format_check_status` 和 `format_check_exit_code`。Codex CLI 可能因为本轮早期工具命令失败而最终返回非零；外层 `advance` 只有在非零退出且文档提交状态有效时才会继续提交 review gate。没有有效文档状态、状态不匹配或产物不完整时必须 block。这个状态头不是 approval，也不能替代 reviewer。

默认 agent backend 是 `codex-cli`，也可以让 Codex CLI 使用指定 API provider：

- `SANDRONE_AGENT_BACKEND=codex-cli`：默认值，调用 Codex CLI。
- `SANDRONE_AGENT_BACKEND=codex-api`：仍然调用 Codex CLI，但临时注入 `model_provider`，让 Codex 使用 `LLM_API_KEY`、`LLM_BASE_URL` 和当前阶段模型。这是 agent 使用 API key/base URL/model 的推荐方式。
- `SANDRONE_AGENT_BACKEND=claude-code`：保留值，默认脚本暂未实现；若设置会阻塞。

可以按阶段覆盖 backend：`SANDRONE_DECOMPOSITION_AGENT_BACKEND`、`SANDRONE_PLAN_AGENT_BACKEND`、`SANDRONE_IMPLEMENTATION_AGENT_BACKEND`。

`codex-api` 保留 Codex 的文件读取、代码编辑、命令执行、sandbox、session 和 reviewer 的 `--output-schema` 能力。它仍然通过 `codex exec` 运行，默认参数包含 `approval_policy="never"`、`shell_environment_policy.inherit="all"` 和 `--sandbox workspace-write`，所以不会弹交互式审批；遇到 sandbox 不允许的写入、命令或网络问题时应失败并进入 block，而不是绕过门禁。默认脚本不再提供脚本直连 API 并代写文件的实现；如果你确实需要其他模型系统，应替换 connector 脚本，并保持同样的输入输出契约。

默认 agent connector 使用 `codex exec --ignore-user-config`，不继承用户个人 Codex config、skill 和插件。Sandrone 会把当前 phase 需要的 prompt、CodeGraph/Obsidian 路径、review detail 路径和脚本能力显式传入 agent；这样可以避免自动化子会话被个人 skill/plugin 强制读入大量无关上下文。

如果某个项目确实需要子 agent 继承个人 Codex skill/plugin，可以在 workspace `.env` 显式设置：

```bash
SANDRONE_AGENT_IGNORE_USER_CONFIG=0
```

关闭隔离后仍要遵守 prompt 的分层读取策略：优先读取当前 status、当前 phase 主产物、CodeGraph context、Obsidian 当前 index、agent journal 最近几轮，以及启动 prompt 列出的最新 review detail；不要一次性扫描完整 skill、完整 project vault、全部 review 历史或全部 slice 文档。

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

`code-review` 会先同步运行 `--check`。失败时不会派发 TestReviewer/DesignReviewer，而是把摘要写入 `status.json.reason`、事件流和 change-doc frontmatter，然后把 request/slice 回退到 implementation。`--check` 通过后，reviewer worker 才会异步派发。

## Reviewer 脚本

默认 reviewer：

- `tools/decomposition-review.sh`
- `tools/plan-review.sh`
- `tools/test-review.sh`
- `tools/design-review.sh`
- `tools/integration-review.sh`

stdout 必须是符合 `tools/schemas/review-result.schema.json` 的 JSON 对象。非法 JSON、空输出、schema 不匹配或脚本失败都会成为 blocking review。

reviewer 命令是异步的：`sdr plan-review`、`sdr code-review`、`sdr integration-review` 只负责创建 attempt、派发 worker 并返回；worker 完成后通过 hook 调用 `advance` 收敛。后台状态和日志在 `.sandrone/state/jobs/<REQ>/<stage>/<attempt>/<reviewer>/`，包含 pid、exit、stdout/stderr、hook、events 和 runtime 元数据；最终结构化结果仍写入 `obsidian/changes/**/reviews/<stage>/details/`。

每个 reviewer 的 `SANDRONE_REVIEW_CONTEXT` 是轻量索引目录，不复制完整 plan、change-doc 或 Obsidian 长文档。框架会自动生成：

- `artifact-index.md`：唯一入口，列出权威原始路径、读取顺序、禁止路径和 slice/request 说明。
- `changed-files.txt`：从 worktree git status/diff 自动生成。
- `diff-stat.txt`：从 worktree diff stat 自动生成。
- `test-summary.txt`：从 change-doc 的验证相关内容和路径信息生成的轻量摘要。

默认 reviewer prompt 会要求先读 `artifact-index.md`。`SANDRONE_PLAN`、`SANDRONE_CHANGE_DOC` 等环境变量仍会指向原始 Obsidian 文件，主要用于兼容自定义 connector，不是默认上下文展开方式。对 slice 来说，没有独立 `request.md`，plan 就是 slice 的权威 request+plan。

默认脚本支持这些 backend：

- `SANDRONE_REVIEW_BACKEND=codex-cli`：默认值，调用 Codex CLI，并为 reviewer 创建隔离的临时 `CODEX_HOME`。
- `SANDRONE_REVIEW_BACKEND=codex-api`：调用 Codex CLI，并让 Codex 使用 `LLM_API_KEY`、`LLM_BASE_URL` 和当前 reviewer 模型；仍然保留 `--output-schema` 结构化输出。
- `SANDRONE_REVIEW_BACKEND=claude-code`：保留值，默认脚本暂未实现；若设置会返回 `gate_unavailable=true`。

可以按 reviewer 类型覆盖 backend：`SANDRONE_DECOMPOSITION_REVIEWER_BACKEND`、`SANDRONE_PLAN_REVIEWER_BACKEND`、`SANDRONE_TEST_REVIEWER_BACKEND`、`SANDRONE_DESIGN_REVIEWER_BACKEND`、`SANDRONE_INTEGRATION_REVIEWER_BACKEND`。

backend 解析优先级：agent 是阶段专用 backend -> `SANDRONE_AGENT_BACKEND` / `SANDRONE_BACKEND` -> 默认 `codex-cli`；reviewer 是类型专用 backend -> `SANDRONE_REVIEWER_BACKEND` -> `SANDRONE_REVIEW_BACKEND` -> 默认 `codex-cli`。

`codex-api` 通用变量：

- `LLM_API_KEY`：API key。
- `LLM_BASE_URL`：API root，例如 `https://api.openai.com/v1` 或兼容 `/v1` 的 provider。
- `SANDRONE_CODEX_MODEL_PROVIDER`：`codex-api` 的临时 provider id，默认 `sandrone-api`。
- `SANDRONE_CODEX_PROVIDER_NAME`：`codex-api` 的 provider 显示名，默认 `Sandrone API`。
- `SANDRONE_CODEX_WIRE_API`：`codex-api` 的 Codex wire API，默认 `responses`。
- `SANDRONE_CODEX_MODEL_CATALOG_JSON`：可选，指向 Codex `model_catalog_json` 文件；未设置时脚本优先使用 `$CODEX_HOME/models_cache.json` 或 `$HOME/.codex/models_cache.json`，否则用 `codex debug models --bundled` 生成临时 catalog。默认 `codex-cli` 和 `codex-api` 都会设置这个值，避免 Codex 启动时现场刷新模型列表，也避免第三方 `/models` 返回格式不兼容导致 Codex 启动失败。
- `SANDRONE_REVIEW_TIMEOUT_SECONDS`：reviewer 子进程超时，默认 `1800`。超时会被转换成 `gate_unavailable=true` 的 blocking review，避免后台 worker 无限运行；`advance`/`tick` 收敛时会把它标记为 blocked。
- `SANDRONE_*_REVIEWER_MODEL`、`SANDRONE_REVIEWER_MODEL` 或 `SANDRONE_MODEL`：选择 reviewer 模型。

API key 只能放在本地未提交的 `.env` 或 shell 环境中，不要写入文档、review detail 或仓库。

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
