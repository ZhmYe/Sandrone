# Plan: Upgrade Default Mode

## 目标与顺序

1. 扩展测试，先固定 `new`、普通 `upgrade` 和 `upgrade --default` 的差异。
2. 为默认运行资产建立 target/example 映射。
3. 修改 `upgrade` 参数解析，支持 `--default`。
4. 普通 `upgrade` 只刷新 example 和输出说明。
5. `--default` 从刷新后的 example 覆盖正式运行资产。
6. 更新 README、workflow skill 和 proposal 索引。

## 实现位置

- `src/main.rs`: `upgrade_workspace`、默认资产映射、example 刷新和 default 覆盖函数。
- `tests/cli_flow.rs`: new/upgrade/upgrade default 集成测试。
- `README.md` 与 `skills/sandrone/SKILL.md`: 说明升级语义。

## 兼容性

普通 `upgrade` 更保守，不再为缺失的正式 connector/prompt/schema 自动写默认文件。旧 workspace 如果希望直接使用默认实现，需要显式运行 `sandrone upgrade --default`。

## 测试策略

- 失败路径先看见: 新测试在实现前因缺少 `--default` 和普通 upgrade 提醒失败。
- 成功路径验证: 相关测试通过后再跑完整 Rust 测试。
- 全量验证包含格式、编译、clippy、测试、proposal 校验和 diff 空白检查。
