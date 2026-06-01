# Upgrade Preserve Agent Journal Spec

## 背景

旧 workspace 运行 `codex-auto-dev upgrade` 后，部分 request 的 `agent-journal.md` 被重写成初始模板，导致历史执行记录丢失。原因是 journal 初始说明里包含 `agent 每轮`，升级逻辑把这段正常说明误判为“可覆盖模板标记”。

## 需求

- 普通 `upgrade` 不得覆盖已经存在的 `agent-journal.md` 历史记录。
- 即使 journal 仍包含默认说明文本，只要文件存在且不是旧 handoff/prompt 文档，也应保留原样。
- 缺失或空的 journal 仍可由升级补齐。
- 需要回归测试覆盖包含默认说明和实际 attempt 的 journal。

## 非目标

- 不恢复已经被旧版本覆盖的历史内容。
- 不改变 plan、change-doc 的升级迁移策略。
