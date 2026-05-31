# Plan: PR Body Review Findings

## 实施步骤

1. 先扩展 finish 集成测试，在 PR body 捕获文件中断言 warning/info finding 的具体内容。
2. 在 `write_pr_body` 中插入平台中立的 `自动评审意见` section。
3. 增加 review detail 渲染辅助函数，基于 summary attempt 读取最终 reviewer JSON。
4. 增加轻量 JSON array/object 解析，提取 finding 字段，不引入额外依赖。
5. 更新 README、workflow skill 和 proposal 索引。
6. 运行格式、编译、clippy、测试、proposal 校验和 diff 检查。

## 改动位置

- `src/main.rs`: PR body 生成、review finding 渲染、轻量 JSON 数组解析。
- `tests/cli_flow.rs`: finish PR body 集成测试。
- `README.md`: PR body 评审意见说明。
- `skills/codex-auto-dev-workflow/SKILL.md`: skill 中的 finish 契约说明。
- `docs/proposals/2026-05-31/pr-body-review-findings/`: 本次框架变更记录。

## 风险与兼容

- 不改变 `tools/pr-create.sh` contract；它仍只接收 body file。
- 不改变 reviewer schema 和 gate 判定，只复用已有 detail JSON。
- 旧 workspace 如果没有 review detail，PR body 会显示缺少自动评审结果，不会阻断 finish。
