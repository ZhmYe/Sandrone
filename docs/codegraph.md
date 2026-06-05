# CodeGraph

CodeGraph 用来给目标仓库建立代码索引，并生成 agent/reviewer 可复用的代码上下文。它的目标是减少盲读代码和重复分析。

## 安装

安装脚本会尽力执行：

```bash
npm install -g @colbymchenry/codegraph
codegraph install -t codex -l global -y
```

如果自动安装失败，可以手动运行上面的命令，或指定 CLI：

```bash
export SANDRONE_CODEGRAPH_BIN=/absolute/path/to/codegraph
```

## Workspace 内的两个产物

| 路径 | 说明 |
| --- | --- |
| `dev/repo/.codegraph` | CodeGraph 索引目录，供 CodeGraph MCP/CLI 查询目标仓库。 |
| `obsidian/codegraph/context.md` | 框架生成的代码上下文摘要，供 agent/reviewer 优先读取。 |

`.codegraph` 是索引，`context.md` 是可读上下文。二者都需要关注。

## 自动时机

框架会在这些时机尝试初始化或刷新：

- `new --url` clone 非空仓库后。
- 计划或拆解前 preflight。
- `start` 前确认目标仓库基线时。
- `sdr obsidian-refresh` 或相关 upgrade 时刷新导航。

如果 CodeGraph 不可用，流程不应 panic。agent/reviewer 应在 journal 或 finding 中记录风险，并给出恢复命令。

## 手动初始化

```bash
sdr doctor
codegraph init -i dev/repo
mkdir -p obsidian/codegraph
codegraph context -p dev/repo "Summarize architecture, entry points, tests, risks, and likely files for sandrone planning" > obsidian/codegraph/context.md
```

检查：

```bash
codegraph status dev/repo
```

## Agent 使用建议

agent 不应该每次从头扫描整个仓库。推荐顺序：

1. 读当前 request/slice 的 Obsidian index。
2. 读 `obsidian/derived/*.json` 和 `dag.json`。
3. 读 `obsidian/codegraph/context.md`。
4. 只针对当前计划涉及的模块，用 CodeGraph 或代码搜索深入。

如果某个 issue 导致大范围重构，agent 应在 plan 或 change-doc 中说明是否需要重新初始化或刷新 CodeGraph context。

## 常见问题

### `codegraph` 命令不存在

```bash
npm install -g @colbymchenry/codegraph
codegraph install -t codex -l global -y
```

或：

```bash
export SANDRONE_CODEGRAPH_BIN=/absolute/path/to/codegraph
```

### 索引缺失

```bash
codegraph init -i dev/repo
```

### context 过期

重新生成：

```bash
codegraph context -p dev/repo "Summarize architecture, entry points, tests, risks, and likely files for current request planning" > obsidian/codegraph/context.md
```
