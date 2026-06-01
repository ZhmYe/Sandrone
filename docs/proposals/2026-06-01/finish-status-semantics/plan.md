# Finish 状态语义收敛计划

## 目标

1. 在状态加载、状态 rank、terminal 判断和 dashboard 阶段判断里统一 canonical status。
2. 让 code-review / integration-review 通过后直接写入 `wait-update-pr`。
3. 调整 finish / pr-status 的状态推进:
   - 创建或复用 PR 成功: `wait-finish`。
   - PR connector 失败: `wait-update-pr`。
   - PR status `merged`: `finished`。
   - PR status `open`: `wait-finish`。
   - PR status `missing` 或 `closed`: `wait-update-pr`。
4. 更新 README、skill 和相关 proposal 文档。
5. 用 CLI flow 测试覆盖 dashboard、finish、pr-refresh、状态同步和 legacy 兼容。

## 风险点

- 旧 workspace 的 `.codex-auto-dev/state/requests.tsv` 和 `docs/changes/*/status.json` 可能仍写着旧状态，因此加载和同步都必须做 canonical 兼容。
- Dashboard 使用全局 registry，状态计数依赖 `load_requests()`；刷新 registry 时必须看到新语义。
- `finished` legacy 修正不能误把未合并 PR 当成完成。

## 测试计划

- `cargo test --test cli_flow`
- `python3 scripts/validate_proposals.py`
- `cargo build`
- `git diff --check`
