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

每个 workspace 会生成 `agents/config/<kind>.json` 和 `.env`。推荐在 `agents/config/<kind>.json` 里配置每种 agent/reviewer 的 `agent_backend`、`model`、`reasoning_effort`、`api_key` 和 `base_url`；`.env` 主要作为旧配置兼容和 workspace 级别兜底。常用 `.env` 变量：

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
shell 环境变量 -> agents/config/<kind>.json -> workspace .env -> Codex 默认配置
```

如需指定其他 env 文件：

```bash
export SANDRONE_ENV_FILE=/absolute/path/to/workspace/.env
```

默认 reviewer connector 会为每次评审创建临时 `CODEX_HOME`：只复制 `auth.json`，并写入禁用插件和 hooks 的最小 `config.toml`。这样可以避免用户全局 Codex 插件缓存缺失、GitHub 限流或网络同步造成 reviewer `gate_unavailable=true`。只有明确设置 `SANDRONE_REVIEW_CODEX_HOME` 时，reviewer 才会使用你提供的完整 Codex home。

默认 agent connector 会使用 `codex exec --ignore-user-config`，不继承用户个人 Codex 配置、skill 和插件。实现 agent 仍会收到 Sandrone phase prompt、CodeGraph/Obsidian 路径、review detail 路径和脚本能力；CodeGraph/Obsidian 推荐通过 workspace 文件与 CLI 使用，而不是依赖个人 Codex skill 自动加载。

如果某个项目确实需要子 agent 继承个人 Codex skill/plugin，可以在 workspace `.env` 显式设置：

```bash
SANDRONE_AGENT_IGNORE_USER_CONFIG=0
```

关闭隔离后要特别注意上下文预算，避免 agent 读入完整 skill、插件说明、全部 review 历史或整座 Obsidian vault。

### Agent / Reviewer 使用 Codex API provider

默认 agent 和 reviewer backend 都是 `codex-cli`。如果希望某些无人值守环节用指定 API key/base URL/model，可以在对应 `agents/config/<kind>.json` 里设置 `agent_backend: "codex-api"`、`api_key`、`base_url` 和 `model`。它仍然启动 Codex CLI，因此保留读文件、改代码、运行命令、sandbox 和结构化输出能力；默认使用 `approval_policy="never"`，不会要求人工审批。

也可以用 `.env` 作为兼容兜底：

```bash
SANDRONE_AGENT_BACKEND=codex-api
SANDRONE_REVIEW_BACKEND=codex-api
LLM_API_KEY=sk-...
LLM_BASE_URL=https://api.openai.com/v1
SANDRONE_AGENT_MODEL=gpt-5.5
SANDRONE_REVIEWER_MODEL=gpt-5.5
SANDRONE_CODEX_MODEL_PROVIDER=sandrone-api
SANDRONE_CODEX_PROVIDER_NAME=Sandrone API
SANDRONE_CODEX_WIRE_API=responses
SANDRONE_REVIEW_TIMEOUT_SECONDS=1800
```

也可以按阶段/类型拆开配置，例如 `SANDRONE_PLAN_AGENT_BACKEND=codex-api`、`SANDRONE_IMPLEMENTATION_AGENT_BACKEND=codex-cli`、`SANDRONE_TEST_REVIEWER_BACKEND=codex-api`、`SANDRONE_DESIGN_REVIEWER_BACKEND=codex-cli`。模型同理可以用 `SANDRONE_PLAN_AGENT_MODEL`、`SANDRONE_IMPLEMENTATION_AGENT_MODEL`、`SANDRONE_TEST_REVIEWER_MODEL`、`SANDRONE_DESIGN_REVIEWER_MODEL` 等覆盖。

支持的 backend 值：

- `codex-cli`：默认，调用 Codex CLI。
- `codex-api`：调用 Codex CLI，但临时配置 `model_provider`，让 Codex 使用 `LLM_API_KEY`、`LLM_BASE_URL` 和当前阶段解析出来的模型。默认 `wire_api=responses`，可用 `SANDRONE_CODEX_WIRE_API` 覆盖。
- `claude-code`：保留值，默认脚本暂未实现；若设置会阻塞，不会绕过流程。

默认脚本不再提供脚本直连 API 并代写文件/评审 JSON 的实现；需要其他 provider 时优先使用 `codex-api`，或替换 connector 脚本。API key 只允许放在未提交的 `.env` 或 shell 环境中，不要写入 plan/change-doc/review detail，也不要提交到目标仓库。

默认 `codex-cli` 和 `codex-api` 都会自动给 Codex CLI 设置 `model_catalog_json`：优先使用 `SANDRONE_CODEX_MODEL_CATALOG_JSON`、`$CODEX_HOME/models_cache.json` 或 `$HOME/.codex/models_cache.json`，否则用 `codex debug models --bundled` 生成临时 catalog。这样 reviewer/agent 启动时不需要现场刷新模型列表，也可以避免兼容 provider 的 `/models` 返回 `{ "data": [...] }` 但 Codex 期待 `{ "models": [...] }` 时，在模型刷新阶段失败或长时间重试。

若 key、模型、网络、base URL 或结构化输出失败，agent/reviewer 会阻塞流程而不是绕过门禁。

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

如果使用内部平台，可以替换 `tools/issue-update.sh`、`tools/pr-create.sh`、`tools/pr-status.sh`、`tools/merge-plan.sh` 和 `tools/pr-merge.sh`，只要遵守 [connectors.md](connectors.md) 的契约即可。
