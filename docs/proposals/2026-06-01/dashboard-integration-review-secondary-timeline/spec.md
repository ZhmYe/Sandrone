# Dashboard Integration Review Secondary Timeline Spec

## 背景

Dashboard 的 PR Refresh 与 Integration Review 支线如果塞在主线右侧，会显得拥挤，也容易和 `Code Review -> Finish / PR` 主流程混在一起。用户希望无 rebase/integration review 时不显示支线；如果存在 Integration Review，则用一条单独的下方 timeline 展示 PR 后集成刷新流程。

## 需求

- 没有 `integration-review` 详情时，不显示 `PR Refresh` 和 `Integration Review` 节点。
- 存在 `integration-review` 详情时，主 timeline 只显示到 `Code Review`。
- 下方居中显示 `PR Refresh -> Integration Review -> Finish / PR`。
- 从上方 `Code Review` 拉一条中性灰虚线曲线到下方 `PR Refresh`，并且虚线应根据节点真实位置计算，避免视口变化后歪斜、横穿文字或延伸到 `Finish / PR` 区域。
- Integration Review 通过后 request 回到 `wait-update-pr` 时，`Finish / PR` 必须作为当前待操作节点展示，因为还需要再次运行 `finish` 推送 rebase 后的分支。
- `PR Refresh` stage 的核心证据优先来自 `pr-conflicts/attempts/*.md`；这些文件只记录真实 rebase 冲突 attempt。没有冲突 attempt 时，stage 才回退展示 `change-doc.md` 中抽取出的 PR 集成刷新章节。
- 点击下方节点仍只展示当前 stage 的核心文件或 review detail。
- 不改变 dashboard JSON 结构，不新增前端依赖。

## 非目标

- 不在前端新增复杂的 PR conflict attempt 交互列表；冲突 attempt 作为该 stage 的 Markdown artifact 串联展示。
- 不改变 request 状态机、reviewer gate 或后端 stage 生成。
