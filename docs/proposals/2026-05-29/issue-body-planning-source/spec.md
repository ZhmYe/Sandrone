# 规格: Issue Body Planning Source

## 背景

默认 issue connector 已经把 GitHub issue 的标题、正文和 URL 输出给框架，但计划阶段的模板和提示词没有足够明确地区分“标题”和“描述”。这会让自动化 Codex 误以为标题就是完整需求。

## 用户目标

每个 issue 的标题和描述都必须进入 `issue.md`，并且 planning thread 必须明确把两者都作为需求来源。标题只能作为概览，不能替代完整需求描述。

## 功能要求

- 默认 GitHub issue connector 必须使用 `--method GET` 拉取 issue，避免 `-f state=open` 导致 `gh api` 默认切换成 POST。
- 默认 GitHub issue connector 必须输出 issue body。
- `issue.md` 必须明确拆分 `需求标题` 和 `需求描述`。
- `codex-plan.md` 和 `thread-handoff.md` 必须明确要求标题和描述都参与计划。
- `tasks.md` 必须提醒 Codex 不得只根据标题写计划。
- README 和 skill 必须同步说明 connector 输出 title/body，以及 planning 阶段必须读两者。

## 非目标

- 不在本次拉取 issue comments。
- 不改变 `requests.tsv` 的字段结构。
- 不实现自动创建 Codex thread；每个 issue 独立会话仍由 automation prompt 或机器人层处理。

## 验收标准

- 默认 `tools/issue-update.sh` 包含 `--method GET`、`--paginate` 和 `(.body // "")`。
- 通过 connector 写入 body 后，`plan` 生成的 `issue.md` 包含需求标题和需求描述。
- planning prompt 和 handoff 明确写出“标题和描述都必须作为需求来源”。
