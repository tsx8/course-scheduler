# AGENTS.md

## General Rules
- 先读后改：修改前先看相关实现、调用方和配置，不要只凭 README 判断。
- 输出简洁，分析完整；不要用讨好式开场或收尾。
- 优先做小而完整的改动；不要顺手重构无关代码。
- 优先编辑现有文件；除非文档基线整体失效，不要整文件重写。
- 不要反复重读已确认未变化的文件。
- 完成前做与改动规模匹配的验证。
- 用户指令永远高于本文件。

## Commands
先按任务范围选择命令，不要无差别全跑。

```bash
npm --prefix frontend install
uv --directory solver sync
uv --directory solver run pyinstaller solver.spec
npm --prefix frontend run build
go build ./...
wails dev
wails build -nsis
```

Notes:
- `wails dev` 是常驻 GUI 开发命令；用于真实交互冒烟检查。
- `wails build -nsis` 会生成 `build/bin/*-installer.exe`。
- `wails.json` 的 `preBuildHooks` 会在打包前构建 solver；hook 运行目录是 `build/bin`，所以路径使用 `../../solver`。
- 本仓库没有自动化测试框架；不要主动引入测试框架或 CI 流水线，除非用户要求。

## Project Overview
- Windows 桌面应用：`Vue 3 + Wails 2 + Go + SQLite + Python OR-Tools`。
- 桌面宿主入口：`main.go`；本地后端：`internal/backend/`。
- 前端子项目：`frontend/`；前端只能通过 `frontend/src/host/desktop.js` 调用 Wails 后端。
- 求解器是 Python sidecar：`solver/solver.py` 经 PyInstaller 输出到 `solver/dist/solver.exe`。
- SQLite 使用主表 / `_temp` 表机制承载编辑态、保存、提交和回滚。

## Important Paths
| Area | Path | Notes |
|---|---|---|
| Frontend entry | `frontend/index.html`, `frontend/src/main.js`, `frontend/src/App.vue` | Vite + Vue 挂载链路 |
| Routing/layout | `frontend/src/router/index.js`, `frontend/src/layouts/MainLayout.vue` | 页面路由、侧边栏、保存/撤销、自动排课、关闭确认 |
| Host adapter | `frontend/src/host/desktop.js` | Wails 调用映射；不要引入其他宿主适配 |
| State | `frontend/src/stores/data.js` | 高风险：自动保存、提交/回滚、课表聚合和 CRUD |
| Pages | `frontend/src/pages/*.vue` | 教师、课程、场地、设置、校区/教师课表 |
| Backend contract | `internal/backend/models.go` | `AllData` JSON 契约源头 |
| Persistence | `internal/backend/storage.go`, `internal/backend/import_export.go` | SQLite 初始化、temp 表、导入导出、兼容迁移 |
| Solver bridge | `internal/backend/solver.go`, `solver/solver.py`, `solver/solver.spec` | sidecar 构建、调用、结果回写 |
| Packaging | `wails.json`, `.github/workflows/build.yml`, `build/windows/installer/project.nsi` | Wails 构建、Windows CI、用户级 NSIS 安装包 |

## Data and Domain Invariants
- 3NF 关系表是主模型，不要降级成布尔、逗号字符串或 JSON 持久化字段。
- 关系表职责：`teacher_courses` 教师-课程，`course_venues` 课程-场地，`teacher_campuses` 教师-校区。
- 前端组合字段只是视图：教师 `teaches`、教师 `campus_ids`、课程 `place` 必须落回关系表。
- `teacher_campuses` / `teacher_campuses_temp` 是教师上课校区的运行时模型。
- `is_only_shahe` 只允许作为 `internal/backend/storage.go`、`internal/backend/import_export.go` 的旧数据兼容输入，不得回流主模型。
- 修改 `AllData` 时同步检查：`models.go`、`storage.go`、`import_export.go`、`frontend/src/stores/data.js`、`solver/solver.py`。

## Validation Matrix
| Change | Minimum validation |
|---|---|
| Documentation only | 无需构建；检查命令和路径是否真实 |
| Frontend page/router/state | `npm --prefix frontend run build` |
| Solver/model contract | `uv --directory solver run pyinstaller solver.spec`，必要时再跑 `go build ./...` |
| Go/Wails/backend | `go build ./...` |
| Packaging/CI/installer | `wails build -nsis` |
| Real interaction flow | `wails dev` 并在 GUI 中走通相关流程 |

## Git and Generated Files
- 提交信息沿用 Conventional Commits，例如 `fix: ...`、`refactor: ...`、`docs: ...`。
- 不要提交生成目录：`frontend/dist/`、`frontend/wailsjs/`、`frontend/node_modules/`、`node_modules/`、`solver/.venv/`、`solver/build/`、`solver/dist/`、`build/bin/`。
- `build/windows/installer/project.nsi` 是跟踪文件；`build/windows/installer/` 里的其他文件是 Wails/NSIS 生成物。
- `frontend/package.json.md5` 是 Wails 本地缓存标记，已忽略，不要重新纳入 Git。

## Boundaries
### Always
- 先定位入口、状态、后端方法、数据结构和构建链路，再修改。
- 对纳入范围的功能做完整闭环：调用方、契约、存储、构建、README、AGENTS 一起更新。
- 保持 Wails-only 宿主链路；前端调用面以 `frontend/src/host/desktop.js` 为准。
- 数据层改动后确认加载、保存、提交、回滚、导入导出仍一致。

### Ask First
- 变更 SQLite 主表 / `_temp` 表机制本身。
- 修改导入导出支持范围或兼容策略。
- 调整 solver 约束语义，而不只是适配现有数据模型。
- 改动锁文件或大规模依赖：`frontend/package-lock.json`、`go.sum`、`solver/uv.lock`。
- 删除当前任务未点名的跨层模块或用户数据。

### Never
- 不要编辑生成目录；唯一例外是明确维护 `build/windows/installer/project.nsi`。
- 不要恢复历史中已删除的目录、宿主链路或功能，除非用户明确要求。
- 不要只改 UI 而漏掉宿主、后端、数据契约或文档。
- 不要把 `teacher_campuses` 简化为布尔字段、字符串字段或 JSON 字段。
- 不要为了构建通过而删除无关逻辑、跳过验证或留下失真的文档。

## Final Checklist
- `README.md` 与 `AGENTS.md` 是否仍描述当前仓库事实。
- `frontend/src/host/desktop.js` 是否仍与页面调用面一致。
- `internal/backend/models.go`、`storage.go`、`import_export.go` 是否仍与 `AllData` 契约一致。
- `solver/solver.py` 是否仍按 `teacher_campuses` 集合约束建模。
- `.github/workflows/build.yml`、`wails.json`、`build/windows/installer/project.nsi` 是否仍能产出用户级安装包。
