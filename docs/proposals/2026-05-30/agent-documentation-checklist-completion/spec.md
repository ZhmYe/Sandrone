# Spec: Agent Documentation Checklist Completion

## 背景

implementation agent 完成代码后，最终交付物不只包含代码和测试，也包含目标项目文档、`change-doc.md` 和 reviewer 可追溯证据。如果文档里保留未勾选 checklist，后续人工审批和自动化机器人会难以判断这是遗漏、阻塞，还是后续流程。自动流程需要明确要求 agent 在结束前关闭交付文档中的 checklist，并把当前流程无法完成的事项移到单独章节。

## 目标

- implementation agent prompt 明确要求完成开发后更新相关目标项目文档和 `change-doc.md`。
- 所有交付文档中的 checklist 必须全部打勾。
- 无法由当前流程完成的事项不得留在未勾选 checklist 中，必须移到后续流程、人工事项或阻塞项。
- runtime `change-doc.md` 模板提供专门的“文档与 Checklist”和“后续流程”章节。
- workflow skill 和 README 同步记录该交付规则，确保 Codex 使用 skill 时能读到同一要求。

## 非目标

- 不自动扫描或修改目标项目文档中的 checkbox。
- 不修改已批准 plan 的 approval 语义；implementation agent 不应为了凑勾篡改 approved plan。
- 不新增 reviewer 类型。

## 行为要求

- implementation agent 退出前必须检查本轮新增或修改的交付文档、`change-doc.md` 和目标项目内部要求文档。
- 如果 checklist 条目已经完成，可以保留并打勾。
- 如果条目需要人工审批、外部发布、账号权限、跨团队确认或后续版本处理，应移到 `后续流程`、`人工事项`、`阻塞项` 或同等章节。
- 不得把尚未真实完成的事项标成已完成。
- `change-doc.md` 必须说明更新了哪些文档、检查了哪些 checklist，以及未完成事项如何追踪。

## 验证

- 新 workspace 生成的 `tools/prompts/implementation-agent.md` 必须包含文档与 checklist 要求。
- `plan` 生成的 `change-doc.md` 模板必须包含“文档与 Checklist”和“后续流程”章节。
- 安装后的 workflow skill 必须包含交付文档 checklist 规则。
