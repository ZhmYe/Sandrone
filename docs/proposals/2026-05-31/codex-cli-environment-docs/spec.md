# Spec: Codex CLI Environment Docs

## 背景

自动流程中的 `tools/issue-agent.sh` 和 reviewer connector 会启动 `codex exec`。当 `sandrone tick` 从普通终端、heartbeat、cron、LaunchAgent 或 GUI 调度器启动时，运行环境不一定包含 Codex App 的 CLI 路径，导致 agent 可能因为找不到 `codex` 而 block。

## 目标

- 在 README 中明确 Codex CLI 的解析顺序: `SANDRONE_CODEX_BIN`、PATH 中的 `codex`、`SANDRONE_CODEX_APP`。
- 给出推荐的 `~/.zshrc` 配置块，避免用户写死只能在单机生效的二进制路径。
- 说明 `~/.zprofile`、bash 配置文件、GUI/LaunchAgent 和 `launchctl setenv` 的适用场景。
- 说明代理变量可以在启动 `tick` 的同一 shell 中配置，并会被子 Codex agent 继承。
- 保持实现为文档变更，不修改 runtime 状态机、不改变 connector 行为。

## 非目标

- 不创建跨重启自动加载的 LaunchAgent 模板。
- 不改变默认 reviewer 或 issue-agent 脚本。
- 不要求所有用户都使用 Codex App 路径；自定义 backend 仍可通过 connector 替换。

## 验收标准

- README 包含环境变量配置章节。
- 文档覆盖终端、GUI 和代理三类常见启动环境。
- README 通过 `git diff --check`。
- proposal 索引通过 `scripts/validate_proposals.py`。
