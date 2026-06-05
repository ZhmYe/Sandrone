# 计划: Explicit Approval, Thread Registry And Upgrade

## 目标依赖图

1. 显式 approval 数据结构与命令。
   这是 `start/finish` 强制门禁的前置条件。
2. `start/finish` 门禁校验。
   依赖 approval 文件和 artifact hash。
3. session registry。
   依赖 request 状态与 change path，供 handoff、机器人和未来前端使用。
4. `upgrade` 迁移旧 workspace。
   依赖新的 schema、approval 目录、session registry 和模板写入规则。
5. 中文模板、skill 和 README。
   依赖前面命令语义稳定后更新，避免文档与实现不一致。

## 代码改动

- 修改 `src/main.rs`:
  - 新增 `SessionRecord`。
  - 新增 `submit`、`approve`、`reject`、`approvals` 命令。
  - 新增 `session`、`sessions` 命令。
  - 新增 `upgrade` 命令。
  - 新增 approval JSON 写入、读取、hash 校验和 stale 检查。
  - 让 `start` 检查 `plan` approval。
  - 让 `finish` 检查 `change-doc` approval。
  - 让 `plan` 生成 `approvals/` 和中文模板。
  - 让 handoff 包含 workspace 绝对路径、目标仓库、仓库来源、原始需求、approval 文件、允许/禁止动作。
  - 将 `change-doc.md` 模板调整为实现说明导向，包含实现前后对比、关键设计点和变更范围摘要。
- 修改 `tests/cli_flow.rs`:
  - 覆盖 approval required、approval stale、finish gate、session registry、upgrade 和中文模板。
- 修改 `skills/sandrone/SKILL.md`:
  - 写明 CLI 验证、显式审批、thread registry、upgrade、中文文档规范和 change-doc 写作标准。
- 修改 `README.md`:
  - 用中文描述命令、流程和升级方式。
- 修改 `proposal.json`:
  - 添加本 proposal 索引。

## 兼容性与迁移

`config.toml` 新增 `schema_version = 2`。旧 config 没有该字段时按版本 1 读取，`upgrade` 写回版本 2。

旧 workspace 通过 `sandrone upgrade --dry-run` 预览迁移，再用 `sandrone upgrade` 应用迁移。迁移只补缺或替换仍为框架默认模板的文件，不覆盖用户已填写内容。

## 测试策略

- 集成测试验证 `start` 在 plan approval 缺失时失败。
- 集成测试验证 plan approval 后修改 artifact 会 stale。
- 集成测试验证 `finish` 在 change-doc approval 缺失时失败。
- 集成测试验证 `session` 能登记 thread URL，`sessions --json` 能输出。
- 集成测试验证 `upgrade` 补齐 sessions/approvals/schema，并保留用户 issue connector。
- 全量运行 `cargo fmt --check`、`cargo check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 和 proposal 校验。

## 风险与回滚

- 第一版 JSON 解析为轻量实现，只解析本工具生成的一行对象。后续如果机器人写入复杂 JSON，应引入正式 JSON crate。
- SHA-256 通过系统 `shasum` 或 `sha256sum` 计算。若目标环境两者都不存在，approval 命令会失败并给出错误。
- `upgrade` 替换默认模板时依赖模板特征文本判断。若用户文档仍保留默认模板标记，可能被视为可替换文件；用户填写后应删除模板提示语。
