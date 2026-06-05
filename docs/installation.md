# 安装与环境

## 依赖

| 工具 | 是否必须 | 说明 |
| --- | --- | --- |
| Git | 必须 | clone、worktree、branch、push。 |
| Rust/Cargo | 源码安装必须 | `cargo install --path .` 安装 CLI；Rust 目标项目默认检查也会使用。 |
| Codex CLI | 默认必须 | 默认 agent/reviewer connector 会调用 `codex exec`。 |
| GitHub CLI `gh` | 默认 GitHub connector 必须 | `issue-update.sh`、`pr-create.sh`、`pr-status.sh` 默认使用。 |
| Node/npm | 推荐 | 自动安装 CodeGraph 时使用。 |
| CodeGraph CLI | 推荐 | 为目标仓库生成 `.codegraph` 索引和 `obsidian/codegraph/context.md`。 |
| Obsidian | 可选 | 打开 workspace 内的 `obsidian/` vault，查看关系图、Base、Canvas。 |

## 安装方式

远程安装：

```bash
curl -fsSL https://raw.githubusercontent.com/ZhmYe/Sandrone/master/scripts/bootstrap.sh | sh
```

本地源码安装：

```bash
scripts/install.sh --force
```

安装脚本会：

- 安装 `sandrone` skill。
- 安装随框架打包的 `obsidian-change-trace` skill。
- 尽力执行 `npm install -g @colbymchenry/codegraph`。
- 尽力执行 `codegraph install -t codex -l global -y`。
- 除非传 `--skill-only`，否则执行 `cargo install --path .`。

注意：`scripts/install.sh --force` 的 `--force` 只表示覆盖已安装 skill。如果要强制刷新已安装 CLI，请运行：

```bash
cargo install --path . --force
```

可选参数：

```bash
scripts/install.sh --skill-only --force
scripts/install.sh --cli-only
scripts/install.sh --dest "$HOME/.codex" --force
```

如果暂时不想自动安装 CodeGraph：

```bash
SANDRONE_SKIP_CODEGRAPH_INSTALL=1 scripts/install.sh --force
```

安装 skill 或 CodeGraph MCP 配置后，建议重启 Codex App。

## 验证

```bash
sdr --help
sandrone --help
sdr doctor
```

`sdr` 是 `sandrone` 的短别名。任何命令都可以互换。

## Codex CLI 路径

默认 connector 查找 Codex CLI 的顺序：

1. `SANDRONE_CODEX_BIN`：直接指定 `codex` 可执行文件，或指定一个能在 `PATH` 中找到的命令名。
2. 当前 `PATH` 中的 `codex`。
3. `SANDRONE_CODEX_APP`：指向 Codex App bundle，例如 `/Applications/Codex.app`。

zsh 常用配置：

```bash
# BEGIN Sandrone
export SANDRONE_CODEX_APP="/Applications/Codex.app"
if [ -d "/Applications/Codex.app/Contents/Resources" ]; then
  case ":$PATH:" in
    *":/Applications/Codex.app/Contents/Resources:"*) ;;
    *) export PATH="/Applications/Codex.app/Contents/Resources:$PATH" ;;
  esac
fi
# END Sandrone
```

写入 `~/.zshrc` 后重新打开终端，或执行：

```bash
source ~/.zshrc
codex --version
```

不同启动方式建议：

| 启动方式 | 建议配置位置 |
| --- | --- |
| zsh 交互终端 | `~/.zshrc` |
| zsh 登录 shell | `~/.zprofile` |
| bash | `~/.bashrc` 或 `~/.bash_profile` |
| GUI 调度器、LaunchAgent | `launchctl setenv` 或用户级 LaunchAgent |

当前 macOS 登录会话可以执行：

```bash
launchctl setenv SANDRONE_CODEX_APP "/Applications/Codex.app"
```

## 模型路由

每个 workspace 会生成 `.env`，默认来自 `templates/.env.example`。常用变量：

```bash
SANDRONE_DECOMPOSITION_AGENT_MODEL=
SANDRONE_PLAN_AGENT_MODEL=
SANDRONE_IMPLEMENTATION_AGENT_MODEL=
SANDRONE_REBASE_AGENT_MODEL=

SANDRONE_PLAN_REVIEWER_MODEL=
SANDRONE_TEST_REVIEWER_MODEL=
SANDRONE_DESIGN_REVIEWER_MODEL=
SANDRONE_INTEGRATION_REVIEWER_MODEL=
```

每个字段都有对应的 `*_REASONING_EFFORT`。读取优先级：

```text
阶段专用变量 -> 通用 agent/reviewer 变量 -> SANDRONE_MODEL -> Codex 默认配置
```

如需指定其他 env 文件：

```bash
export SANDRONE_ENV_FILE=/absolute/path/to/workspace/.env
```

默认 reviewer connector 会为每次评审创建临时 `CODEX_HOME`：只复制 `auth.json`，并写入禁用插件和 hooks 的最小 `config.toml`。这样可以避免用户全局 Codex 插件缓存缺失、GitHub 限流或网络同步造成 reviewer `gate_unavailable=true`。只有明确设置 `SANDRONE_REVIEW_CODEX_HOME` 时，reviewer 才会使用你提供的完整 Codex home。

## 代理

在运行 `sdr tick` 的同一个 shell 中设置代理即可。默认 agent/reviewer 脚本会继承环境变量。

```bash
export https_proxy=http://127.0.0.1:7890
export http_proxy=http://127.0.0.1:7890
export all_proxy=socks5://127.0.0.1:7890
```

## GitHub CLI

默认 GitHub connector 需要：

```bash
gh auth status
gh repo view --json nameWithOwner -q .nameWithOwner
```

如果使用内部平台，可以替换 `tools/issue-update.sh`、`tools/pr-create.sh` 和 `tools/pr-status.sh`，只要遵守 [connectors.md](connectors.md) 的契约即可。
