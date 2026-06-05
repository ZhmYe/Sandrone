# Plan: Agent Documentation Checklist Completion

## 目标与顺序

1. 先补集成测试，覆盖 skill、implementation agent prompt 和 runtime change-doc 模板。
2. 更新 `src/main.rs` 中的默认 issue-agent prompt、implementation-agent prompt 和 change-doc 模板。
3. 更新 `skills/sandrone/SKILL.md` 和 README 的关键规则。
4. 新增 proposal 记录并更新 `proposal.json`。
5. 运行格式、编译、clippy、测试、proposal 校验和 diff 检查。

## 实现位置

- `src/main.rs`: 默认 prompt 和 `render_change_doc_template`。
- `tests/cli_flow.rs`: 生成内容和 skill 内容断言。
- `README.md`: 使用者可见的关键规则。
- `skills/sandrone/SKILL.md`: Codex 使用 skill 时读取的规则。
- `docs/proposals/2026-05-30/agent-documentation-checklist-completion/`: 本次框架自身变更记录。

## 设计说明

规则放在 implementation agent prompt 中，因为只有 implementation 阶段会形成最终交付文档和目标项目文档。issue-agent 通用契约也加入一条边界，避免未来替换 connector 时丢失该规则。change-doc 模板提供固定章节，让 agent 有地方记录检查结果和后续流程。

不要求 implementation agent 修改已批准 plan。plan 是审批产物，篡改会让 approval 语义变混乱；如果执行结果与 plan checklist 不一致，最终 `change-doc.md` 解释即可。

## 测试策略

- TDD red: 先加入断言，确认现有生成内容缺少 checklist 规则。
- TDD green: 更新生成器和文档后，`cargo test --test cli_flow` 通过。
- 回归验证: 运行完整 Rust 检查和 proposal 校验，确保新增规则不破坏现有工作区生成、upgrade 和 review 流程。
