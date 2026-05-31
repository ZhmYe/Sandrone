# 规格: Review Gate Unavailable Block

## 背景

自动 issue-agent 流程中，reviewer gate 是进入实现和完成交付的硬门禁。当前当 `tools/plan-review.sh`、`tools/test-review.sh` 或 `tools/design-review.sh` 失败时，CLI 会生成 blocking review JSON，但 request 仍只是 review rejected。issue-agent 可能误判为计划或实现内容需要修改，从而反复重试，甚至尝试绕过门禁。

## 用户目标

当 reviewer 后端不可用、脚本失败、空输出或非法 JSON 时，框架必须把它识别为 gate unavailable，并立即将 request 标记为 `blocked`。issue-agent 不得修改 reviewer、不得调用 `approve/reject`、不得伪造 approval；只能记录原因并停止。

## 功能要求

- review summary 必须记录 `gate_unavailable` 和诊断信息。
- reviewer 脚本不存在、退出非 0、空输出、非法 JSON 时，`gate_unavailable` 必须为 `true`。
- 自定义 reviewer 可以通过输出 `gate_unavailable: true` 显式声明门禁不可用。
- `plan-review` 遇到 gate unavailable 时必须写入 review detail、summary、status、recovery，并把 request 标记为 `blocked`。
- `code-review` 遇到 gate unavailable 时必须同样 block。
- issue-agent prompt 必须要求先读取 summary；发现 `gate_unavailable: true` 时立即 block。
- issue-agent prompt 和默认脚本契约必须明确禁止调用 `approve/reject` 或修改 approval JSON 绕过 reviewer。

## 非目标

- 不改变 reviewer 正常返回 critical/high 时的修复循环。
- 不移除人工命令，但自动 issue-agent 不得使用人工 approve/reject 作为 reviewer gate 的替代。
- 不绑定某个具体 LLM reviewer 后端。

## 验收标准

- reviewer 后端失败时，命令 stderr 包含具体 reviewer 和 gate unavailable 原因。
- `reviews/<stage>/summary.json` 包含 `gate_unavailable: true` 和诊断摘要。
- request 状态变为 `blocked`，并生成 `recovery.md`。
- 现有正常 reviewer rejected/approved 流程不受影响。
