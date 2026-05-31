# Plan: Upgrade Reference Examples

## 目标与顺序

1. 先用集成测试固定行为: 用户正式脚本不能被覆盖，dry-run 必须显示刷新 example，upgrade 后 example 存在。
2. 抽出默认 connector/prompt/schema 内容函数，让正式默认文件和 example 文件共享来源。
3. 在 `new` 和 `upgrade` 路径刷新 reference examples。
4. 更新 README、workflow skill 和 proposal 索引。
5. 运行格式、编译、clippy、测试和 proposal 校验。

## 实现方式

- 新增 example 路径常量，例如 `tools/issue-update.example.sh`、`tools/plan-review.example.sh`、`tools/prompts/plan-reviewer.example.md` 和 `tools/schemas/review-result.example.schema.json`。
- 新增 `refresh_default_reference_examples()`，它只写 `.example.*` 文件。
- 保留 `write_default_*` 的“如果正式文件存在则跳过”语义。
- 复用 `default_issue_tool_content()`、`default_issue_agent_tool_content()`、`default_pr_tool_content()`、`default_review_tool_content()` 和 `default_review_schema_content()`。

## 兼容性

这是向后兼容变更。旧 workspace 的正式 connector 不会被替换；新增的 example 文件只提供参考和测试入口。若用户曾经编辑 `.example.*`，upgrade 会覆盖它，因为 `.example.*` 被定义为框架维护文件。

## 测试策略

- 集成测试覆盖 dry-run 文案、正式脚本保护和 example 内容。
- 全量运行 `cargo test`，确保 tick、advance、review、finish 流程不受影响。
- 运行 clippy 防止抽函数后产生未使用或风格问题。
