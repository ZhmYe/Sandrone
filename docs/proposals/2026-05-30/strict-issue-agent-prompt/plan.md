# 计划: Strict Issue Agent Prompt

## 目标依赖图

1. 扩写 issue-agent prompt。
   先把 planning、implementation、review loop、journal 和 block 规则写成具体标准。
2. 同步 skill 和 README。
   说明 issue-agent 在提交 reviewer 前必须做自检并记录 finding 处理。
3. 补测试。
   断言默认生成的 issue-agent prompt 包含关键章节。
4. 验证。
   跑完整 Rust 和 proposal 校验。

## 代码改动

- 修改 `src/main.rs`:
  - 扩写 `default_issue_agent_prompt`。
  - 增加启动前检查、journal 格式、plan/change-doc 交付标准、测试验证要求和 block 规则。
- 修改 `tests/cli_flow.rs`:
  - 断言新 workspace 生成的 `tools/prompts/issue-agent.md` 包含关键章节。
- 修改 README 和 skill:
  - 补充 issue-agent 自检和 finding 处理要求。

## 测试策略

- 运行默认 asset 生成测试，确认新 prompt 被写入。
- 运行完整 `cargo test`，确认 tick/review/finish 流程不受影响。
- 运行 proposal 校验和 diff check。

## 风险与回滚

- prompt 变长会增加 issue-agent 上下文，但能减少 reviewer 拒绝和无效轮次。
- 如果后续发现重复内容过多，可以抽出公共 prompt 模板，但本次先保持单文件简单可靠。
