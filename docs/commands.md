# 命令参考

所有命令都可以用 `sandrone` 或短别名 `sdr`。

## Workspace

| 命令 | 作用 |
| --- | --- |
| `sdr new --url <git-url>` | 初始化外框架并 clone 目标仓库到 `dev/repo`。 |
| `sdr new --name <project-name>` | 初始化本地空目标仓库，适合原型。 |
| `sdr doctor` | 检查 workspace、Git、Codex CLI、GitHub CLI、CodeGraph、connector、schema 和事件目录。 |
| `sdr validate` | 检查已有 request 是否具备必要 runtime 文档。 |
| `sdr upgrade --dry-run` | 预览旧 workspace 升级内容。 |
| `sdr upgrade` | 升级 schema、session registry、example、runtime 文档和 registry，迁移阶段文档 frontmatter，不覆盖正式 connector。 |
| `sdr upgrade --default` | 刷新 `.example.*` 后，用默认实现覆盖正式 connector、prompt 和 schema。 |
| `sdr obsidian-refresh` | 重新同步 Obsidian 导航、derived JSON、Base 和 Canvas。 |

## Request

| 命令 | 作用 |
| --- | --- |
| `sdr update` | 调用 `tools/issue-update.sh`，新增或刷新 request；按 external ID 去重。 |
| `sdr list` | 列出当前 workspace 的 request。 |
| `sdr status` | 输出 workspace 基本信息和状态计数。 |
| `sdr status REQ-0001` | 输出单个 request 的来源、状态、文档路径、分支和 worktree。 |
| `sdr doc-status --request_id REQ-0001` | 读取当前阶段文档 frontmatter，快速查看文档提交状态、format-check 摘要和 gate 状态。 |
| `sdr doc-status --request_id REQ-0001 --phase implementation` | 指定读取 decomposition、planning 或 implementation 阶段文档状态；rebase 是旧 workspace 兼容 phase。 |
| `sdr sessions` | 列出 session registry。 |
| `sdr sessions --json` | JSON 输出 session registry。 |
| `sdr session --request_id REQ-0001 --phase planning --thread_id <id>` | 手动登记可见会话信息。 |

## 公开入口

| 命令 | 作用 |
| --- | --- |
| `sandrone loop start --interval-seconds 900` | 后台周期运行自动化 loop。 |
| `sandrone loop restart [--request_id REQ-0001]` | 从 blocked 恢复；不指定 request 时恢复所有 blocked request，之后用 `loop start` 继续自动化。 |
| `sandrone loop stop [--force]` | 请求停止 loop worker；默认软停止，`--force` 只终止 loop worker，不强杀正在运行的 agent/reviewer。 |
| `sandrone loop stop --request_id REQ-0001 --reason "<reason>"` | 主动把一个 request 标记为 blocked。 |
| `sandrone dashboard` | 启动本地监控页面。 |

## Advanced/Internal

下面命令主要给 hook、connector、测试和故障恢复使用。普通使用应优先走 `sandrone loop start/restart/stop` 和 `sandrone dashboard`。

| 命令 | 作用 |
| --- | --- |
| `sdr tick` | loop worker 内部单轮入口：扫描全部 request，刷新已结束 agent/reviewer 状态，在并发上限内派发下一步 agent 或收敛 review。 |
| `sdr tick --request_id REQ-0001` | 只处理一个 request。 |
| `sdr advance --request_id REQ-0001` | 推进单个 request；通常由 agent 或 review worker hook 自动调用。 |

## 手动门禁

自动流程通常不需要手动执行这些命令，但它们适合调试和恢复。

| 命令 | 作用 |
| --- | --- |
| `sdr decompose --name <YYYY-MM-DD-name> --request_id REQ-0001` | 创建父 request 拆解文档、slice DAG 和 Obsidian 导航。 |
| `sdr submit --request_id REQ-0001 --gate decomposition` | 提交 decomposition gate。 |
| `sdr decomposition-review --request_id REQ-0001` | 派发 DecompositionReviewer worker 并返回。 |
| `sdr plan --name <YYYY-MM-DD-name> --request_id REQ-0001` | 兼容入口：创建计划文档包。自动 slice 流程通常由 agent 填写 slice plan。 |
| `sdr submit --request_id REQ-0001 --gate plan` | 提交 plan gate。 |
| `sdr plan-review --request_id REQ-0001` | 派发 PlanReviewer worker 并返回。 |
| `sdr start --request_id REQ-0001` | 在 plan gate 有效后创建 worktree 和分支。 |
| `sdr submit --request_id REQ-0001 --gate change-doc` | 提交 change-doc gate。 |
| `sdr code-review --request_id REQ-0001` | 同步运行格式检查；通过后派发 TestReviewer 和 DesignReviewer worker 并返回。 |
| `sdr integration-review --request_id REQ-0001` | 旧 workspace 兼容入口；新流程使用 `pr-status` 退回 implementation/code-review。 |
| `sdr gates --request_id REQ-0001` | 查看 gate 状态；状态来自对应阶段 Markdown frontmatter，`approvals` 是兼容别名。 |
| `sdr gates --request_id REQ-0001 --json` | JSON 查看 gate 状态，便于机器人读取。 |
| `sdr approve --request_id REQ-0001 --gate plan --by <actor>` | 人工批准 gate。 |
| `sdr reject --request_id REQ-0001 --gate plan --by <actor>` | 人工拒绝 gate。 |

## 交付与恢复

| 命令 | 作用 |
| --- | --- |
| `sdr block --request_id REQ-0001 --stage implementation --reason "<reason>"` | 显式标记 blocked 并写入 recovery。 |
| `sdr resume --request_id REQ-0001` | 从 blocked 恢复到可派发状态；gate 不可用会重跑 reviewer，代码/文档问题会回到 agent 修复。 |
| `sdr finish --request_id REQ-0001 --message "feat: ..."` | 校验 gate，commit、push 分支并调用 PR connector。 |
| `sdr pr-status --request_id REQ-0001` | 调用 PR 状态脚本；只有 `merged` 才标记 finished。 |
| `sdr pr-merge --request_id REQ-0001` | 自动合并执行器；通常由 loop 调用，只有 `pr-status=safe` 且 `change-doc` gate 有效时才调用 merge connector。 |
| `sdr pr-refresh --request_id REQ-0001` | 旧 workspace 兼容入口；新流程会把 PR 状态问题退回 implementation，由下一轮 loop 处理。 |

## Dashboard

| 命令 | 作用 |
| --- | --- |
| `sdr dashboard` | 启动本地监控页面，默认监听 `127.0.0.1:47217`。 |
| `sdr dashboard --host 127.0.0.1 --port 47220` | 指定监听地址和端口。 |
| `sdr dashboard --port 0` | 使用系统分配空闲端口。 |
| `sdr dashboard --json` | 输出 dashboard 数据模型，适合测试、机器人或未来前端复用。 |
