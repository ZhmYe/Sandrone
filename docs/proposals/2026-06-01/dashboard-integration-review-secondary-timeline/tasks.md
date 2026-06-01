# Dashboard Integration Review Secondary Timeline Tasks

- [x] 普通模式隐藏 `PR Refresh` 和 `Integration Review`。
- [x] 有 Integration Review 详情时启用双层 timeline。
- [x] 下层居中展示 `PR Refresh -> Integration Review -> Finish / PR`。
- [x] 从 `Code Review` 到 `PR Refresh` 增加按节点位置计算的 SVG 虚线曲线。
- [x] Integration Review 通过后，`wait-update-pr` 的 `Finish / PR` 作为当前待操作节点展示。
- [x] 支线轨道、虚线连接和未完成的 `Finish / PR` 不使用 warning/amber 色，避免误判为冲突或警告。
- [x] `PR Refresh` stage 优先展示 `pr-conflicts/attempts/*.md` 中的真实冲突记录；无冲突时仅抽取 change-doc 的 PR 刷新章节。
- [x] 保持 stage 点击和 artifact 展示逻辑。
- [x] 更新 README、skill、proposal 索引和 HTML 单测。
