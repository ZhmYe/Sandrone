# 变更文档: PR Body Review Findings

## 摘要

本次变更让 `finish` 生成的 PR 描述包含具体自动评审意见。人类 reviewer 在 GitHub PR 页面可以直接看到 PlanReviewer、TestReviewer、DesignReviewer 的 warning/info 和其他 finding，不需要回到本地 JSON 文件查找。

## 实现前后对比

- 实现前: PR body 只有关联需求、request 和 change-doc。reviewer 的具体 finding 留在 `reviews/<stage>/details/*.json`，PR 页面不展示。
- 实现后: PR body 增加 `自动评审意见` section，从最终 review attempt 的 detail JSON 汇总 reviewer decision、summary、detail path 和各 severity finding。

## 关键设计点

### 平台中立 PR Body

PR body 仍由 Rust 框架生成，`tools/pr-create.sh` 只负责把 body file 交给 GitHub、GitLab、Gerrit 或内部系统。这样不同平台不需要重复实现 review JSON 解析。

### 只展示最终 Review Attempt

渲染逻辑读取 `reviews/<stage>/summary.json` 中的 `attempt`，再定位对应 reviewer detail。PR 描述展示的是最终状态，不把历史被打回的轮次混在一起，避免人类评审误判。

### Finding 具体化

每条 finding 展示 title、evidence、impact、required_fix、suggested_change 和 verification。即使 gate approved，warning/info 也会进入 PR 描述，作为人类 reviewer 的关注点。

## 变更范围摘要

- CLI: `write_pr_body` 增加自动评审意见渲染。
- JSON 处理: 增加轻量数组/object 提取函数读取 reviewer finding。
- 测试: finish 集成测试增加 PR body warning/info 断言。
- 文档: README、workflow skill、本 proposal。

## 目标项目内部要求

- 已阅读的目标项目文档: README、workflow skill、finish/PR delivery 代码和集成测试。
- 目标项目 change doc: 本文件。
- Pre-commit: Not required，项目没有独立 pre-commit 脚本。
- 文档检查: `python3 scripts/validate_proposals.py`。
- Format/lint/test: `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- AI review: Not required。
- 所有目标项目内部要求是否完成: yes。

## 文档与 Checklist

- 已更新的文档: README、workflow skill、本 proposal。
- 所有交付文档中的 checklist 是否已全部打勾: yes；检查范围包括本 proposal 的 `tasks.md`、本 `change-doc.md`、README 和 workflow skill。
- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: yes；本次没有剩余人工事项。

## 后续流程

本次没有需要保留的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。

## 验证证据

- TDD red: `cargo test --test cli_flow finish_requires_change_doc_approval_then_commits_and_pushes_request_branch -- --nocapture` 失败，PR body 不包含 `自动评审意见`。
- TDD green: 实现 review finding 渲染后，上述测试通过。
- `cargo fmt --check` 通过。
- `cargo check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test` 通过，34 个集成测试全部通过。
- `python3 scripts/validate_proposals.py` 通过，验证 23 个 proposal。

## Review 结果

本次框架自身变更没有运行外部 reviewer gate；后续以本地格式、编译、clippy、测试、proposal 校验和 diff 检查作为交付证据。
