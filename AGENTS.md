# AGENTS.md

## Agent Role
- 你是这个 Wails 桌面排课应用的维护者；不要把前端、Go 后端、SQLite、Python solver 当成孤立模块处理。
- 默认做小而完整的改动：入口、调用方、数据契约、持久化、构建命令和文档要一起闭环。
- 先读后改；修改前至少看相关实现、调用方和配置，不要只凭 README 或记忆判断。

## Commands
先按任务范围选择命令，不要无差别全跑。`justfile` 是开发命令入口；`just` 通过 `uv tool install rust-just` 管理，临时执行可用 `uvx --from rust-just just <recipe>`。

| Task | Command |
|---|---|
| 查看命令 | `just` |
| 安装开发依赖 | `just install` |
| 按 CI 锁文件安装依赖 | `just install-ci` |
| lint / 格式检查 | `just lint` |
| 自动格式化 | `just format` |
| 前端构建 | `just build-frontend` |
| solver sidecar 构建 | `just build-solver` |
| Go 后端构建 | `just build-go` |
| 本地完整构建 | `just build` |
| Wails GUI 开发运行 | `just dev` |
| Windows 安装包 | `just package` |

Notes:
- `just dev` 是常驻 GUI 开发命令，用于真实交互烟测。
- `just package` 调用 `wails build -nsis`，产出 `build/bin/*-installer.exe`。
- `wails.json` 的 `preBuildHooks` 会在打包前从 `build/bin` 运行 `uv --directory ../../solver run pyinstaller solver.spec`。
- CI 仍直接执行底层命令，不依赖 `just`。
- 本仓库没有自动化测试框架；不要主动引入测试框架或 CI 流水线，除非用户要求。

## Working Rules
- 输出简洁，分析完整；不要用讨好式开场或收尾。
- 优先编辑现有文件；除非文档基线整体失效，不要整文件重写。
- 不要反复重读已确认未变化的文件；如果文件可能被工具或并发编辑改动，先重读。
- 不要顺手重构无关代码；删除 obsolete 代码时要确认没有调用方。
- 完成前做与改动规模匹配的验证，并报告实际运行过的命令。
- 用户指令永远高于本文件。

## Project Overview
- Windows 桌面应用：`Vue 3 + Wails 2 + Go + SQLite + Python OR-Tools`。
- 桌面宿主入口：`main.go`；本地后端：`internal/backend/`。
- 前端子项目：`frontend/`；前端只能通过 `frontend/src/host/desktop.js` 调用 Wails 后端。
- 求解器是 Python sidecar：`solver/solver.py` 经 PyInstaller 输出到 `solver/dist/solver.exe`。
- SQLite 使用主表 / `_temp` 表机制承载编辑态、保存、提交和回滚。

## Important Paths
| Area | Path | How to work there |
|---|---|---|
| Frontend entry | `frontend/index.html`, `frontend/src/main.js`, `frontend/src/App.vue` | Vite + Vue 挂载链路，少动入口全局行为。 |
| Routing/layout | `frontend/src/router/index.js`, `frontend/src/layouts/MainLayout.vue` | 页面路由、侧边栏、保存/撤销、自动排课、关闭确认。 |
| Host adapter | `frontend/src/host/desktop.js` | 前端只能通过这里调用 Wails 后端；不要新增平行宿主适配。 |
| State | `frontend/src/stores/data.js` | 高风险：自动保存、提交/回滚、课表聚合和 CRUD。 |
| Diagnostics/drag UI | `frontend/src/stores/scheduleDiagnostics.js`, `frontend/src/stores/scheduleDrag.js`, `frontend/src/pages/ScheduleIssues.vue`, `frontend/src/pages/CampusTimetable.vue`, `frontend/src/pages/TeacherTimetable.vue` | 问题定位、拖拽调整、焦点高亮要保持一致。 |
| Schedule components | `frontend/src/components/Schedule*.vue` | 课程卡、单元格、详情抽屉、暂存托盘共享课表交互语义。 |
| Backend contract | `internal/backend/models.go` | `AllData` JSON 契约源头。 |
| Persistence | `internal/backend/storage.go`, `internal/backend/import_export.go` | SQLite 初始化、主表 / `_temp` 表、导入导出、兼容迁移。 |
| Solver bridge | `internal/backend/solver.go`, `internal/backend/solver_process_*.go`, `solver/solver.py`, `solver/solver.spec` | sidecar 构建、调用、结果回写。 |
| Tooling/CI | `justfile`, `.github/workflows/build.yml`, `.golangci.yml`, `.prettierrc.json`, `frontend/.oxlintrc.json`, `solver/pyproject.toml` | 命令入口、lint/format、CI 版本要同步。 |
| Packaging | `wails.json`, `build/windows/installer/project.nsi` | Wails 构建、用户级 NSIS 安装包。 |

## Architecture and Data Contracts
- SQLite 主表 / `_temp` 表承载编辑态、保存、提交和回滚；数据层改动后要验证加载、保存、提交、回滚、导入导出仍一致。
- 3NF 关系表是主模型，不要降级成布尔、逗号字符串或 JSON 持久化字段。
- 关系表职责：`teacher_courses` 教师-课程，`course_venues` 课程-场地，`teacher_campuses` 教师-校区。
- 前端组合字段只是视图：教师 `teaches`、教师 `campus_ids`、课程 `place` 必须落回关系表。
- `teacher_campuses` / `teacher_campuses_temp` 是教师上课校区的运行时模型。
- `is_only_shahe` 只允许作为 `internal/backend/storage.go`、`internal/backend/import_export.go` 的旧数据兼容输入，不得回流主模型。
- 修改 `AllData` 时同步检查：`internal/backend/models.go`、`internal/backend/storage.go`、`internal/backend/import_export.go`、`frontend/src/stores/data.js`、`solver/solver.py`。
- 调整诊断、拖拽或课表定位时，同步检查 `scheduleDiagnostics.js`、`scheduleDrag.js`、`ScheduleIssues.vue`、两张 timetable 页面和 `Schedule*.vue` 组件。

## Validation Matrix
| Change | Minimum validation |
|---|---|
| Documentation / `AGENTS.md` / `justfile` | `uvx --from rust-just just --fmt --check`，并检查命令和路径真实存在 |
| Frontend page/router/state | `just lint-frontend` + `just build-frontend` |
| Schedule diagnostics / drag UI | `just lint-frontend` + `just build-frontend`，必要时用 `just dev` 做 GUI 烟测 |
| Solver/model contract | `just lint-solver` + `just check-solver-format` + `just build-solver`，必要时再跑 `just build-go` |
| Go/Wails/backend | `just lint-go` + `just build-go` |
| Cross-layer data flow | `just lint` + `just build` |
| Packaging/CI/installer | `just package` |
| Real interaction flow | `just dev` 并在 GUI 中走通相关流程 |

## Tooling and Generated Files
- Frontend lint 使用 Oxlint：`frontend/.oxlintrc.json`，命令由 `frontend/package.json` 暴露为 `npm --prefix frontend run lint`。
- Frontend/config format 使用 Prettier：`.prettierrc.json`；当前只格式化列入 `format:check` 的文件，避免大规模 Vue/JS churn。
- Go lint 使用 `go.mod` 的 `tool github.com/golangci/golangci-lint/v2/cmd/golangci-lint`，入口是 `go tool golangci-lint run` / `just lint-go`。
- Python solver 使用 `uv` 管理依赖，Ruff 配置在 `solver/pyproject.toml`。
- 提交信息沿用 Conventional Commits，例如 `fix: ...`、`refactor: ...`、`docs: ...`。
- 不要提交生成目录：`frontend/dist/`、`frontend/wailsjs/`、`frontend/node_modules/`、`node_modules/`、`solver/.venv/`、`solver/build/`、`solver/dist/`、`build/bin/`。
- `build/windows/installer/project.nsi` 是跟踪文件；`build/windows/installer/` 里的其他文件是 Wails/NSIS 生成物。
- `frontend/package.json.md5` 是 Wails 本地缓存标记，已忽略，不要重新纳入 Git。

## Boundaries
### Always
- 先定位入口、状态、后端方法、数据结构和构建链路，再修改。
- 保持 Wails-only 宿主链路；前端调用面以 `frontend/src/host/desktop.js` 为准。
- 对纳入范围的功能做完整闭环：调用方、契约、存储、构建、README、AGENTS 一起更新。
- 改动锁文件后说明原因，并运行对应安装/同步命令验证锁文件可用。

### Ask First
- 变更 SQLite 主表 / `_temp` 表机制本身。
- 修改导入导出支持范围或兼容策略。
- 调整 solver 约束语义，而不只是适配现有数据模型。
- 改动大规模依赖或工具链版本：`frontend/package-lock.json`、`go.sum`、`solver/uv.lock`、CI action 版本。
- 删除当前任务未点名的跨层模块、历史兼容逻辑或用户数据。

### Never
- 不要编辑生成目录；唯一例外是明确维护 `build/windows/installer/project.nsi`。
- 不要恢复历史中已删除的目录、宿主链路或功能，除非用户明确要求。
- 不要只改 UI 而漏掉宿主、后端、数据契约或文档。
- 不要把 `teacher_campuses` 简化为布尔字段、字符串字段或 JSON 字段。
- 不要为了构建通过而删除无关逻辑、跳过验证或留下失真的文档。

## Final Checklist
- `README.md` 与 `AGENTS.md` 是否仍描述当前仓库事实和命令入口。
- `justfile` 是否仍能列出、lint、build 相关配方。
- `frontend/src/host/desktop.js` 是否仍与页面调用面一致。
- `internal/backend/models.go`、`storage.go`、`import_export.go` 是否仍与 `AllData` 契约一致。
- `solver/solver.py` 是否仍按 `teacher_campuses` 集合约束建模。
- `.github/workflows/build.yml`、`wails.json`、`build/windows/installer/project.nsi` 是否仍能产出用户级安装包。
