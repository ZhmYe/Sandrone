# Change Doc: Codex CLI Environment Docs

## 变更摘要

为 README 增加 Codex CLI 环境变量配置说明，帮助用户在终端、heartbeat、cron、Codex App 或 GUI 调度器场景下稳定启动子 Codex agent 和 reviewer。

## 实现前

- README 只说明安装 CLI 和 skill，没有解释默认 connector 如何找到 `codex`。
- 用户遇到 `codex CLI is not installed` 时，需要从日志反推 PATH、Codex App 和 shell 配置的关系。

## 实现后

- README 明确记录 `CODEX_AUTO_DEV_CODEX_BIN`、PATH、`CODEX_AUTO_DEV_CODEX_APP` 的解析顺序。
- README 提供可复制的 `~/.zshrc` 配置块，并说明 `~/.zprofile`、bash 和 GUI/LaunchAgent 的差异。
- README 说明代理变量应在启动 `tick` 的同一 shell 中设置，并会被子 agent 继承。

## 本机环境调整

- 已备份并更新 `/Users/zhmye/.zshrc`，追加 `codex-auto-dev` 配置块。
- 已为当前 macOS 登录会话设置 `CODEX_AUTO_DEV_CODEX_APP=/Applications/Codex.app`。
- 已把原有 `.zshrc` 中会在非登录环境下报错的 `brew`、`npm`、`nvm` 和 OpenClaw source 行加上存在性检查。

## 验证

- [x] `zsh -ic 'printf "CODEX_AUTO_DEV_CODEX_APP=%s\n" "$CODEX_AUTO_DEV_CODEX_APP"; command -v codex; codex --version'`
- [x] `launchctl getenv CODEX_AUTO_DEV_CODEX_APP`
- [x] `git diff --check -- README.md`
- [x] `python3 scripts/validate_proposals.py`

## 自动评审意见

本次是文档和本机 shell 配置变更，没有运行 reviewer gate。README 文档已通过格式检查，proposal 文档已通过索引校验。
