# 开发本框架

## 源码结构

| 路径 | 说明 |
| --- | --- |
| `src/main.rs` | CLI 命令分发、tick/agent 编排和流程胶水。 |
| `src/state.rs` | `requests.tsv`、`sessions.json`、`status.json`、gate 和事件流读写。 |
| `src/doc_status.rs` | 阶段 Markdown frontmatter、文档提交状态、format/check 摘要、gate 状态和旧状态迁移。 |
| `src/codegraph.rs` | CodeGraph CLI 检查、索引初始化和 context 生成。 |
| `src/obsidian.rs` | Obsidian vault、导航笔记、derived JSON、Base、Canvas。 |
| `src/review_gate.rs` | reviewer gate 执行、JSON 规范化和结果写入。 |
| `src/delivery.rs` | `finish` 阶段 git commit/push、PR body 和 PR connector。 |
| `src/doctor.rs` | 环境诊断。 |
| `src/registry.rs` | 全局 `workspaces.json`。 |
| `src/dashboard.rs` | dashboard HTTP 服务、JSON 数据模型和 artifact 映射。 |
| `src/defaults.rs` | workspace 默认模板、connector、prompt、schema 生成/升级。 |
| `src/assets.rs` | 编译期引用模板和静态资产。 |
| `assets/dashboard/index.html` | dashboard 前端固定资产。 |
| `templates/prompts/*.md` | 默认 agent/reviewer prompt。 |
| `templates/scripts/*.sh` | 默认 connector 脚本模板。 |
| `templates/runtime/*.md` | runtime Markdown 模板。 |
| `templates/schemas/*.json` | structured output schema。 |

## 本仓库治理

源码仓库使用 proposal/change 文档：

```text
proposal.json
docs/proposals/YYYY-MM-DD/<proposal-id>/
  spec.md
  plan.md
  tasks.md
  plan.html
  change-doc.md
```

这些是本框架仓库的开发治理文件，不会复制到用户的 runtime workspace。用户 workspace 的运行时文档在 `obsidian/changes/`。

`docs/proposals/` 是历史归档，不是当前运行规范。旧 proposal 可能仍记录当时的 `approval JSON`、`status.json.gates` 或旧文档目录设计；实现、prompt 和用户操作以 README、`docs/workflow.md`、`docs/workspace-layout.md`、`templates/prompts/` 和 `skills/sandrone/SKILL.md` 为准。

项目 constitution 入口见 [constitution.md](constitution.md)。

## 本地验证

完整验证：

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
python3 scripts/validate_proposals.py
git diff --check
```

文档或 proposal-only 变更至少运行：

```bash
python3 scripts/validate_proposals.py
git diff --check
```

如果改了默认脚本模板，建议额外运行：

```bash
sh -n templates/scripts/*.sh
```

如果改了 dashboard 前端，建议启动：

```bash
sdr dashboard --port 0
```

并用浏览器检查 request 列表、slice tab、review detail、Markdown/JSON 展示和高度滚动。
