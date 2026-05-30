# Schedule Feature Session Queue

Use this queue to run the decomposed requirements in separate Codex conversations. Start a new conversation for each session after restarting Codex so the installed skills are loaded.

## Session 1: Confirm Schedule Version Contract

Skill: `grill-with-docs`

First message:

```text
使用 grill-with-docs，围绕 D:\UserData\Documents\Code\course-scheduler\docs\plans\2026-05-31-schedule-workflow-breakdown.md 的 Slice 1，确认课表版本的数据边界和切换策略。请先读取 AGENTS.md、docs/agents/*、CONTEXT.md 和该拆解文档；只做决策和文档回填，不实现代码。
```

Done when:

- Version boundary is confirmed.
- Unsaved-change switching policy is confirmed.
- `CONTEXT.md` and the Slice 1 section are updated if decisions differ from the draft.
- Final answer names Slice 2 as the next session.

## Session 2: Versioned Storage and Backend Contract

Skill: `zoom-out`, then normal implementation workflow.

First message:

```text
使用 zoom-out，先阅读 D:\UserData\Documents\Code\course-scheduler\docs\plans\2026-05-31-schedule-workflow-breakdown.md 的 Slice 2、AGENTS.md、docs/agents/*、CONTEXT.md，并定位 storage/model/import_export/solver/data store 的调用链。然后实现课表版本存储与后端契约；不要做 UI 管理入口。完成后按拆解文档验证并回填实际表结构、迁移策略和验证命令。
```

Done when:

- Old single-plan database migrates into one default active plan.
- Backend can load/save/commit/revert/import/export active plan data.
- Solver still operates on current active plan.
- Validation commands from Slice 2 pass or failures are explicitly documented.
- Slice 2 section records implementation facts for Session 3.

## Session 3: Create, Copy, and Switch Schedule Versions

Skill: `zoom-out`, then normal implementation workflow. Use `grill-with-docs` only if Slice 1 left a UI decision unresolved.

First message:

```text
使用 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 3，实现课表版本创建、复制和切换 UI。请先读取 AGENTS.md、docs/agents/*、CONTEXT.md 和 Slice 2 的实际实现记录；检查后端契约后再做前端闭环。完成后验证并回填 UI 入口位置和切换行为。
```

Done when:

- User can create a blank schedule version.
- User can copy current schedule version.
- Switching refreshes timetable views, issues, staging tray, and solver input.
- Unsaved changes follow the Slice 1 policy.
- Validation commands from Slice 3 pass or failures are explicitly documented.

## Session 4: Manual Add from Campus Timetable

Skill: `zoom-out`, then normal implementation workflow.

First message:

```text
使用 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 4，在校区总课表实现手动新增排课。请先读取 AGENTS.md、docs/agents/*、CONTEXT.md 和相关 ScheduleCell/CampusTimetable/data store 代码；复用教师课表已有规则，不引入平行数据路径。完成后验证并回填复用路径和残余风险。
```

Done when:

- Campus timetable blank cell can add a schedule.
- Teacher/course/venue options are compatibility-filtered.
- Added schedule appears in both campus and teacher timetable.
- Diagnostics, autosave, save/revert, lock/staging semantics remain consistent.
- Validation commands from Slice 4 pass or failures are explicitly documented.

## Session 5: Campus CSV Three-Column Export

Skill: `zoom-out`, then normal implementation workflow.

First message:

```text
使用 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 5，重做校区总课表 CSV 导出布局：每条排课占据 项目/教师/地点 三个单元格，并保持时间段对齐。请先读取 AGENTS.md、docs/agents/*、CONTEXT.md 和 CampusTimetable.vue 的导出代码；完成后验证并回填最终 CSV 布局和样例检查结果。
```

Done when:

- Export has two-level day/subcolumn headers.
- Every schedule occupies adjacent `项目`、`教师`、`地点` cells.
- Multi-schedule cells expand into aligned rows instead of newline-packed cells.
- Filters still apply.
- CSV escaping is robust.
- Validation commands from Slice 5 pass or failures are explicitly documented.

## Operating Rule

Do not start Session 2 before Session 1 decisions are recorded. Sessions 4 and 5 can run after Session 1 if schedule versioning is not being changed in parallel, but if Sessions 2/3 have already landed, re-read their final notes first.
