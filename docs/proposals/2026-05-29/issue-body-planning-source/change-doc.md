# 变更文档: Issue Body Planning Source

## 摘要

本次变更修正默认 GitHub issue connector 的 GET 调用，并把 issue 标题和描述共同作为 planning 阶段的强制需求来源。Codex 后续生成计划时不能只看标题。

## 实现前后对比

- 实现前: 默认 connector 使用 `gh api` 加 `-f state=open` 但没有显式 `--method GET`；`issue.md` 只在标题中展示 issue title，正文放在 `原始需求`，planning prompt 没有直接强调标题和描述都要参与计划。
- 实现后: 默认 connector 使用 `--method GET --paginate`，继续输出 `.title`、`(.body // "")` 和 `.html_url`。`issue.md` 拆分 `需求标题` 和 `需求描述`，planning prompt 与 handoff 明确写入“标题和描述都必须作为需求来源”。

## 关键设计点

### 默认 Connector

`gh api` 在使用 `-f` 时会默认切到 POST，因此默认脚本改为 `--method GET`。`--paginate` 让 open issue 超过一页时仍然能被拉取。输出格式保持 TSV 五列不变，避免破坏已有状态结构。

### Planning 输入语义

框架仍然不生成真实计划，只生成模板和 handoff。为了避免自动化只看 title，`issue.md` 明确拆出 `需求标题` 和 `需求描述`，`codex-plan.md` 与 `thread-handoff.md` 要求两者都作为需求来源。

## 变更范围摘要

改动集中在默认 issue connector、runtime planning 模板、README、skill 和集成测试。没有改变 request 状态文件结构，也没有新增 comments 拉取。

## 验证证据

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `python3 scripts/validate_proposals.py`

## 风险与后续

- 旧 workspace 中用户自定义的 `tools/issue-update.sh` 不会被覆盖，需要手动同步 GET/body 写法。
- GitHub issue comments 仍未进入需求正文，后续可以把 comments 拉取作为 connector 可选增强。
