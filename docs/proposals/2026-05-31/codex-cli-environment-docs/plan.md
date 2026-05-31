# Plan: Codex CLI Environment Docs

## 实现计划

1. 在 README 安装章节后增加 `Codex CLI 环境变量` 小节。
2. 写明 connector 查找 Codex CLI 的顺序，和 `CODEX_AUTO_DEV_CODEX_APP` 的推荐用法。
3. 给出可直接复制到 `~/.zshrc` 的配置块，并列出 `~/.zprofile`、bash 配置和 GUI/LaunchAgent 场景。
4. 补充代理变量继承说明，解释为什么应在启动 `tick` 的 shell 中设置。
5. 创建 proposal 文档包并更新 `proposal.json`。

## 影响范围

- `README.md`
- `proposal.json`
- `docs/proposals/2026-05-31/codex-cli-environment-docs/`

## 风险

- 文档可能被理解成必须使用 `/Applications/Codex.app`。已通过解析顺序说明保留 `CODEX_AUTO_DEV_CODEX_BIN` 和 PATH 方案。
- `launchctl setenv` 只对当前登录会话有效。README 已明确该限制，并提示跨重启可用用户级 LaunchAgent。

## 验证

- `zsh -ic 'printf "CODEX_AUTO_DEV_CODEX_APP=%s\n" "$CODEX_AUTO_DEV_CODEX_APP"; command -v codex; codex --version'`
- `launchctl getenv CODEX_AUTO_DEV_CODEX_APP`
- `git diff --check -- README.md`
- `python3 scripts/validate_proposals.py`
