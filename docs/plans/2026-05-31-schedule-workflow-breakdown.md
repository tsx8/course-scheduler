# Schedule Workflow Breakdown

## Purpose

把三个新需求拆成后续独立 Codex 会话可以逐个完整执行的工作流单元。每个 Slice 都应该在新的会话中从读取上下文开始，按指定技能工作流走完探索、决策或实现、验证、文档回填和交接，而不是在当前会话里连续实现。

## Initial Code Facts

- 以下事实是拆解时的实施前快照，用于解释为什么按 Slice 拆分；后续 Implementation notes 才记录当前实现结果。
- `internal/backend/models.go` 的 `AllData` 当时只有一组 `scheduled_classes`，没有课表版本元数据。
- `internal/backend/storage.go` 用主表 / `_temp` 表管理编辑态，`scheduled_classes` 和 `scheduled_classes_temp` 是当前唯一排课结果存储。
- `frontend/src/stores/data.js` 持有一套 `scheduledClasses`，所有课表视图、诊断、拖拽、自动保存都围绕这套数据工作。
- `internal/backend/solver.go` 从 `LoadData()` 取当前数据，只把锁定且非暂存的排课传给 Python solver，再把 solver 输出保存回临时表。
- 教师个人课表的新增/编辑排课能力在 `frontend/src/components/ScheduleCell.vue`。
- 校区总课表在 `frontend/src/pages/CampusTimetable.vue` 中直接渲染 `ScheduleCard`，当前禁用了卡片编辑，空白区域只用于拖拽落点和容量显示。
- 校区 CSV 导出在 `CampusTimetable.vue` 的 `handleExportToCsv` 中实现；现在每个时间段/工作日只生成一个 CSV cell，多条排课用换行拼在一个 cell 内。

## New-Session Execution Model

- 每个 Slice 单独开启一个新会话，不和其他 Slice 混跑。
- 新会话第一条消息使用对应的 `Recommended new-session prompt`。
- 每个新会话都先读取 `AGENTS.md`、`docs/agents/*`、本文件、`CONTEXT.md`，再读相关代码。
- 新会话结束前必须更新本文件对应 Slice 的状态或新增补充说明，除非该会话只做口头决策且用户明确不落文档。
- 如果某个 Slice 的实现发现拆解不准确，先修正文档和依赖关系，再继续实现。

## Implementation Order

### Slice 1: Lock the Schedule Version Contract

Type: HITL

Blocked by: None

What to decide:

- 课表版本只版本化排课结果，不复制教师、课程、校区、场地、时间段、工作日和约束基础数据。
- 新增空白课表应复用共享基础数据和界面筛选上下文，但版本内 `scheduled_classes` 为空。
- 复制现有课表只复制当前课表版本的排课记录，并给排课记录生成新 ID。
- 切换课表版本前必须处理未保存更改：推荐先阻止切换并提示保存或撤销，不在切换时自动提交。
- 自动排课、诊断、拖拽、暂存区、CSV 导出都只作用于当前课表版本。

Confirmed decisions:

- 课表版本的数据边界已确认：课表版本只拥有排课记录及排课状态，即 `scheduled_classes` 以及每条记录的 `is_locked`、`is_staged`、`staged_order` 等状态。
- 教师、课程、校区、场地、时间段、工作日、教师不可用时间、排课密度、教师-课程、课程-场地、教师-校区关系，以及已保存的校区筛选视图都属于共享基础数据，不随课表版本复制。
- “新增空白课表”应理解为：在同一套共享基础数据上创建新的空课表版本，版本内没有排课记录；不要复制基础数据表行。
- 当前 UI 的筛选选择可以作为界面状态保留，但不作为课表版本数据持久化。
- 切换课表版本前必须处理未保存更改。已确认策略为：如果存在未保存更改，禁止切换课表版本，并提示用户先保存或撤销；切换动作本身不自动提交，也不提供“丢弃并切换”的快捷路径。
- 复制现有课表时，只复制当前课表版本的排课记录及排课状态，包括锁定状态、暂存状态和暂存顺序；复制出的每条排课记录必须生成新的 `scheduled_classes.id`，不能复用来源版本的排课记录 ID。
- 自动排课、问题检查、拖拽调整、暂存区、校区 CSV 导出、校区总课表和教师个人课表都只作用于当前课表版本。后端 `LoadData()` 应只返回当前课表版本的 `scheduled_classes`，前端 store 不持有所有版本的排课记录全集；版本列表只提供元数据和当前版本标识。
- 当前课表版本必须由后端持久化保存，而不是只存在前端 UI 选择里。`LoadData()`、`RunSolver()`、导入导出和应用重启都应以后端保存的当前版本标识为准；前端选择器只负责发起切换，切换成功后重新加载当前版本数据。
- 因为教师、课程、校区、场地、时间段和工作日是共享基础数据，删除这些基础数据时，所有课表版本中引用它们的排课记录都应一起移除，不能保留悬空排课记录。后续 UI 可以提示该操作会影响所有课表版本。
- JSON 和数据库这类完整数据导出应包含所有课表版本、当前版本标识和共享基础数据；校区 CSV 导出是当前视图报表，只导出当前课表版本。
- 导入完整 JSON 或数据库时，保持替换整个工作区数据的语义，不把导入文件里的课表版本合并进当前数据库。新格式导入应替换共享基础数据、所有课表版本和当前版本标识；旧单版本导入应迁移成一个默认课表版本。导入仍进入现有未保存编辑态，由用户保存或撤销。
- 当没有未保存更改时，切换当前课表版本应立即持久化后端保存的当前版本标识。切换课表版本是导航状态变更，不是排课内容编辑；切换后不要求用户再点“保存更改”来保存当前版本标识。
- “有未保存更改时禁止切换”按整个工作区的未保存更改判断，不区分基础数据变化、当前课表版本排课变化，或导入后的临时数据。只要 `HasUnsavedChanges()` 为 true，就禁止切换课表版本。
- 切换课表版本成功后，应清空拖拽中状态和课表焦点高亮，因为它们绑定具体排课记录 ID；校区/教师/课程/场地筛选选择可以保留，因为它们绑定共享基础数据。暂存课程不是界面状态，而是版本内排课状态，切换后应显示新当前课表版本自己的暂存课程。
- 课表版本元数据只用于版本列表和当前版本管理，不参与 solver、诊断和排课规则。版本元数据最低需要 `id`、`name` 和用于稳定展示的排序或时间字段；当前版本标识单独持久化。solver、诊断、拖拽和 CSV 导出只消费当前课表版本的 `scheduled_classes`。
- 版本管理动作也遵守工作区级未保存拦截。只要 `HasUnsavedChanges()` 为 true，禁止切换、创建空白课表、复制当前课表、重命名课表版本和删除课表版本，并提示用户先保存或撤销。
- 删除课表版本时，禁止删除最后一个版本。允许删除当前课表版本，但必须在同一后端事务里删除该版本的排课记录和版本元数据，并把当前版本标识切换到一个确定的剩余版本，例如按排序或创建时间最靠前的版本；前端随后重新加载。

Acceptance criteria:

- [x] 用户确认上面的版本边界，或明确哪些基础数据也要随版本复制。
- [x] 用户确认“有未保存更改时禁止切换”的策略，或改成保存/丢弃确认弹窗。
- [x] 如有不同决策，同步更新 `CONTEXT.md` 中的术语。
- [x] 会话结束前把确认后的决策写回本 Slice，供 Slice 2/3 使用。

Recommended new-session prompt:

```text
使用 grill-with-docs，围绕 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 1，确认课表版本的数据边界和切换策略。只做决策记录，不实现代码。
```

### Slice 2: Add Schedule Version Storage and Backend Contract

Type: AFK after Slice 1

Blocked by: Slice 1

What to build:

Introduce a versioned schedule model while preserving the existing single-version database as the default plan. The app should load existing data into one default `课表版本`, expose active version metadata through the backend/frontend contract, and keep save/commit/revert semantics coherent for the active version.

Expected shape:

- Add `schedule_plans` / `schedule_plans_temp` metadata tables or equivalent normalized tables.
- Associate `scheduled_classes` / `scheduled_classes_temp` with a schedule plan.
- Add active plan state in a small metadata table rather than inferring it from UI selection.
- Add migration so existing databases get one default active plan and existing rows attach to it.
- Extend `AllData` only as much as the frontend needs for plan listing and active plan display.
- Keep JSON/SQLite import-export compatible with old single-plan exports.

Acceptance criteria:

- [x] Existing databases load without data loss and show one default active plan.
- [x] Save, commit, rollback, import JSON, import database, export JSON, and export database preserve the active plan and plan rows.
- [x] `LoadData()` only returns排课记录 for the current active plan unless the UI explicitly needs all plan metadata.
- [x] `RunSolver()` still operates on the current active plan only.
- [x] Validation: `just lint-go` + `just build-go`; if contract touches frontend store, also `just lint-frontend` + `just build-frontend`.
- [x] 会话结束前在本文件记录实际采用的表结构、迁移策略和验证命令。

Implementation notes:

- 实际表结构：新增 `schedule_plans` / `schedule_plans_temp`，字段为 `id TEXT PRIMARY KEY`、`name TEXT NOT NULL`、`sort_order INTEGER NOT NULL DEFAULT 0`；新增 `app_metadata` / `app_metadata_temp`，字段为 `key TEXT PRIMARY KEY`、`value TEXT NOT NULL`，其中 `active_schedule_plan_id` 保存当前课表版本。
- `scheduled_classes` / `scheduled_classes_temp` 新增 `schedule_plan_id TEXT NOT NULL DEFAULT 'default-schedule-plan'`，新建库带到 `schedule_plans(id)` / `schedule_plans_temp(id)` 的 `ON DELETE CASCADE` 外键，并增加 `schedule_plan_id` 索引。旧库通过 `ALTER TABLE ... ADD COLUMN schedule_plan_id TEXT` 迁移后补值。
- 迁移策略：`initDatabase()` 创建新表后运行课表版本迁移；当版本表为空时创建 `id='default-schedule-plan'`、`name='默认课表'` 的默认课表版本；已有排课记录缺少 `schedule_plan_id` 时挂到当前有效版本，缺少当前版本元数据时写入默认或排序最靠前版本。`_temp` 表只在检测到已有未保存编辑态时补默认版本，避免空临时表导致误判为未保存。
- 迁移回归修复：`just dev` 在旧库上暴露过 `no such column: schedule_plan_id`，原因是索引创建早于旧表补列；当前实现把 `schedule_plan_id` 索引创建移到课表版本迁移之后，并用 `internal/backend/storage_test.go` 覆盖旧库启动迁移。
- 后端契约：`AllData` 增加 `schedule_plans`、`active_schedule_plan_id`，`ScheduledClass` 增加可选 `schedule_plan_id`。普通 `LoadData()` 只返回当前课表版本的 `scheduled_classes`，但仍返回版本元数据；导入、导出和提交路径使用全量加载以保留所有课表版本的排课记录。
- 保存策略：前端 store 只持有版本元数据和当前课表版本的 `scheduled_classes`。`SaveTempData()` 写入临时表时替换当前课表版本的排课记录，并从现有主表或临时表保留其他课表版本的排课记录；如果共享基础数据被删除，其他版本中引用已删除教师、课程、校区、场地、工作日或时间段的排课记录会被过滤，避免悬空引用。
- 兼容策略：旧 JSON 和旧 SQLite 导入如果没有课表版本表、`active_schedule_plan_id` 或 `schedule_plan_id`，会被解释成一个默认当前课表版本。新 JSON / SQLite 完整导出包含所有 `schedule_plans`、当前版本标识和带 `schedule_plan_id` 的全量 `scheduled_classes`。
- 验证命令：`uvx --from rust-just just lint-go`、`uvx --from rust-just just build-go`、`uvx --from rust-just just lint-frontend`、`uvx --from rust-just just build-frontend`，均已通过。

Recommended new-session prompt:

```text
使用 zoom-out 先阅读 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 2 和相关 storage/model/import_export/solver/data store 代码，然后实现课表版本存储与后端契约。不要做 UI 管理入口，完成后按文档验证。
```

### Slice 3: Add Schedule Version UI for Create, Copy, and Switch

Type: AFK after Slice 2

Blocked by: Slice 2

What to build:

Expose schedule version management in the desktop UI. Users can see the current plan, create a blank plan, copy the current plan, rename/delete where safe, and switch plans under the confirmed unsaved-change policy.

Expected UX:

- Put the selector in a global place that affects both timetable views and auto-schedule, most likely `MainLayout.vue`.
- Create blank plan: no active排课记录, but all shared基础数据 remains available.
- Copy current plan: duplicate排课记录 into a new plan with new IDs.
- Switch plan: reload current active plan data, clear schedule focus/drag state, and update diagnostics.
- Version management actions are blocked whenever there are workspace-level unsaved changes: switch, create blank, copy current, rename, and delete all require saving or reverting first.
- Delete plan: prevent deleting the only remaining plan; define behavior if deleting the active plan.
- Delete active plan: allowed when at least one other plan remains; backend picks a deterministic remaining plan as the new active plan and the frontend reloads it.

Acceptance criteria:

- [x] User can create a blank课表版本 and the timetable becomes empty without deleting teachers/courses/venues.
- [x] User can copy the current课表版本 and edits in the copy do not mutate the source.
- [x] Switching plans updates校区总课表、教师个人课表、问题检查、暂存区 and auto-schedule input.
- [x] Unsaved changes are handled exactly as decided in Slice 1.
- [x] Validation: `just lint-frontend` + `just build-frontend`; run `just build-go` if Wails methods are added or changed.
- [x] 会话结束前在本文件记录最终 UI 入口位置和切换行为。

Implementation notes:

- UI 入口位置：`frontend/src/layouts/MainLayout.vue` 的全局标题栏，在窗口控制按钮左侧增加“课表版本”选择器，以及新建空白、复制当前、重命名、删除当前四个图标按钮；该入口在校区总课表、教师个人课表、问题检查、设置和自动排课入口之间保持全局一致。
- 后端操作面：新增 Wails 方法 `CreateSchedulePlan`、`CopySchedulePlan`、`SwitchSchedulePlan`、`RenameSchedulePlan`、`DeleteSchedulePlan`，前端只通过 `frontend/src/host/desktop.js` 的命令映射调用。所有版本管理动作先检查 `HasUnsavedChanges()` 对应的工作区级 `_temp` 表状态；只要存在未保存更改就拒绝执行，并提示先保存或撤销。
- 创建空白课表版本：后端在 `schedule_plans` 主表新增版本、立即写入 `app_metadata.active_schedule_plan_id`，不写入任何 `scheduled_classes`；前端随后用返回的 `AllData` 重载，所以共享教师、课程、校区、场地等基础数据保留，当前课表版本排课为空。
- 复制当前课表版本：后端复制当前 active plan 的全部排课记录及 `is_locked`、`is_staged`、`staged_order` 状态，给新版本和每条复制出的排课记录生成新 UUID，并把新版本设为当前课表版本；复制后继续只加载新当前版本的排课记录。
- 切换行为：切换成功后后端只更新主表中的当前版本标识；前端通过 `dataStore.switchSchedulePlan()` 重载后端返回的当前版本数据，保留校区/教师/课程/场地筛选选择，清空拖拽状态和课表焦点高亮。校区总课表、教师个人课表、问题检查、暂存区和自动排课输入都继续消费 `scheduledClasses`，因此自动跟随新的当前课表版本。
- 删除行为：前端禁止删除最后一个课表版本；删除当前版本时，后端删除该版本元数据和排课记录，并按 `sort_order, name, id` 加载顺序选择确定的剩余版本作为新当前版本，前端随后重载。
- 前端自动保存保护：版本管理返回的 `AllData` 使用一次性的 backend-data 应用状态写入 store，避免 `replaceAllData()` 被自动保存 watcher 误判成用户编辑并制造 `_temp` 编辑态。
- 验证命令：`uvx --from rust-just just lint-frontend`、`uvx --from rust-just just build-frontend`、`uvx --from rust-just just build-go`、`uvx --from rust-just just lint-go`，均已通过。

Recommended new-session prompt:

```text
使用 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 3，实现课表版本创建、复制和切换 UI。先检查 Slice 2 的后端契约，再做前端闭环和验证。
```

### Slice 4: Add Manual Schedule Creation from Campus Timetable

Type: AFK

Blocked by: None, but rebase after Slice 2/3 if schedule version work has already landed.

What to build:

Give `校区总课表` parity with the teacher timetable for manual course creation. In a campus/time/day cell, the user should be able to add a排课记录 by choosing course, teacher, and venue with compatibility filtering.

Expected behavior:

- The form starts from fixed `campus_id`, `day_id`, and `time_id`.
- Teacher options are constrained by selected course and teacher-campus eligibility.
- Venue options are constrained by selected course and current campus.
- The added schedule uses the same `dataStore.addSchedule` path so autosave, diagnostics, lock state, staging, save/rollback and CSV export stay coherent.
- Consider enabling card edit in campus context only if the same validation rules are reused and no conflicting UI emerges.

Acceptance criteria:

- [x] A blank校区总课表 cell provides an add action without breaking drag/drop or capacity controls.
- [x] The user can create a valid排课记录 from campus view and immediately see it in both校区总课表 and教师个人课表.
- [x] Invalid teacher/course/venue combinations cannot be selected.
- [x] Diagnostics update after manual add.
- [x] Validation: `just lint-frontend` + `just build-frontend`; use `just dev` for a GUI smoke test if feasible.
- [x] 会话结束前在本文件记录校区新增排课复用的数据路径和任何无法 GUI 验证的残余风险。

Implementation notes:

- 复用路径：`frontend/src/pages/CampusTimetable.vue` 的单元格新增按钮打开共享的 `frontend/src/components/ScheduleEditModal.vue`，以 `mode="campus"` 固定 `campus_id`、`day_id`、`time_id`，提交时调用同一条 `dataStore.addSchedule(teacherId, scheduleData)` 路径。新增后的排课记录进入当前课表版本的 `scheduledClasses`，继续由现有 watcher 触发自动保存，诊断、暂存、锁定、保存/回滚、校区 CSV 导出和教师个人课表都消费同一份数据。
- 教师课表复用：`frontend/src/components/ScheduleCell.vue` 已改为使用同一个 `ScheduleEditModal.vue`，以 `mode="teacher"` 保持原先固定教师、选择课程/校区/场地的行为；教师课表的编辑排课也继续走 `dataStore.updateSchedule`。
- 规则复用：`frontend/src/stores/data.js` 新增 `campusCourseOptions`、`courseTeacherOptions`，并把教师-校区可用性集中到 `teacherCanTeachAtCampus`。教师没有显式 `teacher_campuses` 关系时仍按原教师课表规则视为可在全部校区授课；课程必须有当前校区/场地的 `course_venues` 关系，教师必须有选中课程的 `teacher_courses` 关系。
- 筛选继承：校区新增表单继承当前校区视图的教师、课程和场地筛选范围；单场地筛选会把表单场地限制在该场地，避免新增后被当前校区总课表筛选隐藏。
- UI 行为：无筛选时新增按钮和容量控件共用单元格顶部控制行；有筛选时保留一个小的新增入口。按钮阻止 pointer down/up 冒泡，避免和 `ScheduleDropCell` 的拖拽落点处理冲突。
- 验证命令：`uvx --from rust-just just lint-frontend`、`uvx --from rust-just just build-frontend`，均已通过。
- 残余风险：本会话未启动 `just dev` 做真实 GUI 点击烟测；当前结论来自静态实现检查、Oxlint/Prettier 检查和 Vite 生产构建。后续如果要确认弹窗焦点、筛选继承和拖拽落点的真实交互手感，应在 Wails GUI 中走一遍“校区总课表空白单元格新增 -> 教师个人课表可见 -> 问题检查更新”的手工流程。

Recommended new-session prompt:

```text
使用 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 4，在校区总课表实现手动新增排课。复用教师课表 ScheduleCell 的规则，但不要引入平行数据路径。
```

### Slice 5: Export Campus CSV with Three Columns per Schedule Slot

Type: AFK

Blocked by: None

What to build:

Change the `校区总课表` CSV export so each排课记录 occupies three CSV cells: `项目`、`教师`、`地点`. The exported timetable must keep time slots aligned even when one day has multiple schedules in the same time slot and another day has fewer.

Recommended layout:

- Header row 1: `时间`, then each day name spans conceptually across three repeated columns in CSV form, e.g. `星期一`, ``, ``, `星期二`, ``, ``.
- Header row 2: ``, then `项目`, `教师`, `地点` repeated for each day.
- For each time slot, compute the maximum schedule count across all days in that time slot.
- Emit that many rows for the time slot.
- Fill each day triplet by row index; if a day has no schedule at that index, emit three blanks.
- Put the time label in the first row of that time slot and leave subsequent rows blank, or repeat the time label if the user prefers easier spreadsheet filtering.

Acceptance criteria:

- [x] A single schedule appears as three adjacent cells: course/project, teacher, venue/location.
- [x] Multiple schedules in the same time/day produce multiple aligned rows under the same time slot, not newline-packed content inside one cell.
- [x] Empty cells produce three blanks so every row has a stable column count.
- [x] Existing campus/venue/teacher/course filters still affect export.
- [x] CSV escaping handles commas, quotes, and newlines in names.
- [x] Validation: `just lint-frontend` + `just build-frontend`; inspect one generated CSV manually from `just dev` if feasible.
- [x] 会话结束前在本文件记录最终 CSV 布局和人工样例检查结果。

Implementation notes:

- 最终 CSV 布局：第一行是分组表头，第一列为 `时间`，每个工作日占三个连续列，形如 `星期一,,,星期二,,`；第二行第一列为空，每个工作日重复 `项目,教师,地点`。正文按时间段输出，每个时间段先计算所有工作日中最大的排课记录数，输出对应行数；该时间段第一行写时间段名称，后续行第一列留空。
- 每条排课记录占三个相邻 cell：课程名作为 `项目`，教师名作为 `教师`，场地名作为 `地点`。同一时间段中某个工作日没有第 N 条排课时，补 `,,` 三个空 cell，保证每行列数恒定为 `1 + 工作日数 * 3`。
- 过滤路径保持不变：导出继续使用 `CampusTimetable.vue` 的 `tableData`，因此当前校区、场地、教师和课程筛选仍然决定导出范围。
- CSV 转义策略：所有 cell 统一加双引号，内部双引号替换为两个双引号；逗号、引号和换行都保留在合法 CSV cell 内，不再把多条排课用换行拼进同一个 cell。
- 样例检查：用两天一段时间的样例运行布局算法，其中周一有两条排课、周二有一条排课，且名称包含逗号、双引号和换行。结果为 4 行、7 列，所有行列数一致；第二条周一排课占用第二个正文行，周二对应位置补三个空 cell；引号被转义为 `""`，逗号和换行保留在双引号 cell 中。
- 验证命令：`uvx --from rust-just just lint-frontend`、`uvx --from rust-just just build-frontend`，均已通过。未启动 `just dev` 生成真实下载文件；样例检查通过本地 Node 片段验证布局和转义。

Recommended new-session prompt:

```text
使用 docs/plans/2026-05-31-schedule-workflow-breakdown.md 的 Slice 5，重做校区总课表 CSV 导出布局：每条排课占据 项目/教师/地点 三个单元格，并保持时间段对齐。
```

## Suggested Session Sequence

1. Slice 1 first, because it fixes the version boundary and switch policy.
2. Slice 2 next, because database and backend contract are the foundation.
3. Slice 3 after Slice 2, because UI switch/create/copy depends on backend primitives.
4. Slice 4 can run after Slice 3 or independently if urgent; if it lands before versioning, re-check after Slice 3.
5. Slice 5 can run independently and is a good small validation task for the new workflow.

## Workflow Rules for New Sessions

- Start each new session by naming the slice and the skill to use.
- Read this file, `AGENTS.md`, and `docs/agents/*` before modifying code.
- Use `zoom-out` when entering an unfamiliar area, `grill-with-docs` when a product decision is fuzzy, and `diagnose` only for concrete bugs/regressions.
- Do not publish GitHub issues unless the user explicitly asks; this file is the local source of truth for now.
- Keep every implementation vertical: update model/contract/store/UI/solver/import-export/docs together when the slice requires it.
- Use the validation matrix in `AGENTS.md`, and report the exact commands run.
- End each new session with a short handoff note: changed files, validation commands, unresolved decisions, and the next Slice to start.
