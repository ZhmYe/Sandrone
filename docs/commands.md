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
| `sdr upgrade` | 升级 schema、session registry、example、runtime 文档和 registry，不覆盖正式 connector。 |
| `sdr upgrade --default` | 刷新 `.example.*` 后，用默认实现覆盖正式 connector、prompt 和 schema。 |
| `sdr obsidian-refresh` | 重新同步 Obsidian 导航、derived JSON、Base 和 Canvas。 |

## Request

| 命令 | 作用 |
| --- | --- |
| `sdr update` | 调用 `tools/issue-update.sh`，新增或刷新 request；按 external ID 去重。 |
| `sdr list` | 列出当前 workspace 的 request。 |
| `sdr status` | 输出 workspace 基本信息和状态计数。 |
| `sdr status REQ-0001` | 输出单个 request 的来源、状态、文档路径、分支和 worktree。 |
| `sdr sessions` | 列出 session registry。 |
| `sdr sessions --json` | JSON 输出 session registry。 |
| `sdr session --request_id REQ-0001 --phase planning --thread_id <id>` | 手动登记可见会话信息。 |

## 自动推进

| 命令 | 作用 |
| --- | --- |
| `sdr tick` | 扫描全部 request，刷新已结束 agent 状态，在并发上限内派发 agent。 |
| `sdr tick --request_id REQ-0001` | 只处理一个 request。 |
| `sdr tick --parallel-limit 2` | 单次覆盖并发上限。 |
| `sdr tick --max-attempts 20` | 单次覆盖 review 最大修复轮数。 |
| `sdr advance --request_id REQ-0001` | 推进单个 request；通常由 agent wrapper hook 自动调用。 |

## 手动门禁

自动流程通常不需要手动执行这些命令，但它们适合调试和恢复。

| 命令 | 作用 |
| --- | --- |
| `sdr decompose --name <YYYY-MM-DD-name> --request_id REQ-0001` | 创建父 request 拆解文档、slice DAG 和 Obsidian 导航。 |
| `sdr submit --request_id REQ-0001 --gate decomposition` | 提交 decomposition gate。 |
| `sdr decomposition-review --request_id REQ-0001` | 运行 DecompositionReviewer。 |
| `sdr plan --name <YYYY-MM-DD-name> --request_id REQ-0001` | 兼容入口：创建计划文档包。自动 slice 流程通常由 agent 填写 slice plan。 |
| `sdr submit --request_id REQ-0001 --gate plan` | 提交 plan gate。 |
| `sdr plan-review --request_id REQ-0001` | 运行 PlanReviewer。 |
| `sdr start --request_id REQ-0001` | 在 plan gate 有效后创建 worktree 和分支。 |
| `sdr submit --request_id REQ-0001 --gate change-doc` | 提交 change-doc gate。 |
| `sdr code-review --request_id REQ-0001` | 运行格式检查、TestReviewer 和 DesignReviewer。 |
| `sdr integration-review --request_id REQ-0001` | 运行 PR refresh 后的轻量集成门禁。 |
| `sdr gates --request_id REQ-0001` | 查看 gate 状态；`approvals` 是兼容别名。 |
| `sdr gates --request_id REQ-0001 --json` | JSON 查看 gate 状态。 |
| `sdr approve --request_id REQ-0001 --gate plan --by <actor>` | 人工批准 gate。 |
| `sdr reject --request_id REQ-0001 --gate plan --by <actor>` | 人工拒绝 gate。 |

## 交付与恢复

| 命令 | 作用 |
| --- | --- |
| `sdr block --request_id REQ-0001 --stage implementation --reason "<reason>"` | 显式标记 blocked 并写入 recovery。 |
| `sdr resume --request_id REQ-0001` | 从 blocked 恢复到可派发状态，同步 `requests.tsv` 和 `status.json`。 |
| `sdr finish --request_id REQ-0001 --message "feat: ..."` | 校验 gate，commit、push 分支并调用 PR connector。 |
| `sdr pr-status --request_id REQ-0001` | 调用 PR 状态脚本；只有 `merged` 才标记 finished。 |
| `sdr pr-refresh --request_id REQ-0001` | 同步 base/master、rebase、处理冲突并运行 IntegrationReviewer。 |

## Dashboard

| 命令 | 作用 |
| --- | --- |
| `sdr dashboard` | 启动本地监控页面，默认监听 `127.0.0.1:47217`。 |
| `sdr dashboard --host 127.0.0.1 --port 47220` | 指定监听地址和端口。 |
| `sdr dashboard --port 0` | 使用系统分配空闲端口。 |
| `sdr dashboard --json` | 输出 dashboard 数据模型，适合测试、机器人或未来前端复用。 |
