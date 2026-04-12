# AGENTS.md

## General Rules

- 先思考再动手：先读现有文件，再写代码。
- 输出保持简洁，但分析要完整。
- 优先编辑现有文件，不要无故整文件重写；但当文档或迁移基线整体失效时，可以一次性重写。
- 除非文件可能已变化，不要反复重读同一文件。
- 完成前做与改动规模匹配的验证。
- 不要用讨好式开场或收尾。
- 方案保持直接、可回滚、最小闭环。
- **修改与迁移遵循“最小充分彻底原则”**：最小充分，指只做完成用户目标所必需的改动，不擅自扩大范围；彻底，指一旦某项功能进入本次范围，就要沿着入口、状态、命令、数据结构、构建链路和剩余文档清理干净，不留半残实现。
- **用户指令永远高于本文件。**

## 当前仓库现实

- 当前仓库是 **Vue 3 + Wails 2 + Go + SQLite + Python OR-Tools** 的 Windows 桌面应用。
- 桌面宿主与本地后端入口是 `main.go` + `internal/backend/`。
- 前端子项目已经收敛到 `frontend/`；根目录只保留 Go/Wails 主链路与仓库级配置。
- 核心数据流仍是 **SQLite 主表 / `_temp` 表双表机制**。
- 求解器仍是 **Python sidecar**：`solver/solver.py`，经 `PyInstaller` 构建到 `solver/dist/solver.exe`，并由 Go/Wails 直接使用该目录作为 sidecar 单一来源。
- 当前仓库已经移除了登录 / RBAC、审计日志、运行日志系统；不要恢复它们。
- 当前应同步维护的剩余仓库文档只有：`README.md`、`AGENTS.md`。

## 业务与模型基线

- 教师“上课校区”已经完成迁移，运行时主模型是 `teacher_campuses` / `teacher_campuses_temp`。
- 旧 `is_only_shahe` 兼容逻辑只能留在迁移 / 导入边界，不得回流主模型。
- 本仓库继续遵循 3NF：
  - `teacher_courses`：教师-课程
  - `course_venues`：课程-场地
  - `teacher_campuses`：教师-校区
- 不要把多值关系降级成布尔字段、逗号分隔字符串或 JSON 持久化字段。

## 先跑这些命令

```bash
npm --prefix frontend install
uv --directory solver sync
npm run build:solver
npm --prefix frontend run build
go build ./...
wails dev
wails build
```

Windows 本地数据位置：

```bash
powershell -Command "echo $env:APPDATA"
```

应用真实数据目录：`%APPDATA%\com.tsxb.course-scheduler\`
- 数据库：`%APPDATA%\com.tsxb.course-scheduler\course_scheduler.db`
- 求解器临时文件：`solver_input.tmp.json`、`solver_output.tmp.json`
- 解包后的求解器：`solver.exe`
- 当前仓库不再维护 `%APPDATA%\com.tsxb.course-scheduler\logs\`

## 重要路径

### 前端骨架
- `frontend/index.html`：Vite HTML 入口
- `frontend/vite.config.js`：前端构建配置
- `frontend/package.json`：前端依赖与 Vite/Wails 前端脚本
- `frontend/src/main.js`：Vue 挂载入口
- `frontend/src/App.vue`：全局 Naive UI Provider
- `frontend/src/router/index.js`：路由定义
- `frontend/src/layouts/MainLayout.vue`：侧边菜单、保存/撤销、自动排课、关闭确认、自定义标题栏
- `frontend/src/host/desktop.js`：前端宿主适配层；当前应只面向 Wails

### 前端状态层
- `frontend/src/stores/data.js`：核心业务 store，含自动保存、提交/回滚、课表 CRUD、教师/课程/校区/场地变更

### 前端页面
- `frontend/src/pages/TeacherManagement.vue`：教师管理；教师校区模型的主要前端入口
- `frontend/src/pages/CourseManagement.vue`：课程管理
- `frontend/src/pages/VenueManagement.vue`：校区/场地管理
- `frontend/src/pages/TeacherTimetable.vue`：教师课表 + CSV 导出
- `frontend/src/pages/CampusTimetable.vue`：校区总课表 + 密度配置 + CSV 导出
- `frontend/src/pages/Settings.vue`：导入导出、时间段/工作日配置
- `frontend/src/components/ScheduleCell.vue`：教师课表单元格的新增/编辑/删除/不排课

### Go / Wails 后端
- `main.go`：Wails 入口、窗口参数、资源绑定
- `wails.json`：Wails 构建配置
- `internal/backend/app.go`：应用生命周期、对话框、关闭拦截
- `internal/backend/models.go`：`AllData` 契约与数据结构
- `internal/backend/storage.go`：数据库初始化、load/save temp、commit/revert、兼容迁移、核心数据装载
- `internal/backend/import_export.go`：JSON / SQLite 导入导出与旧数据兼容映射
- `internal/backend/solver.go`：solver 调用链路与关闭收尾

### 求解器与脚本
- `solver/solver.py`：排课模型；教师校区集合约束的核心实现点
- `solver/solver.spec`：PyInstaller 配置
- `solver/pyproject.toml`、`solver/uv.lock`、`solver/.python-version`：solver 子项目的 Python 依赖与版本元数据
- `solver/dist/`：sidecar 单一产物来源；Go embed 与运行时磁盘探测都以此为准
- `solver/scripts/prepare-solver.js`：校验 `solver/dist/solver.exe` 已就绪，并清理旧 `build/sidecar/` 残留
- `package.json`：repo 级脚本入口；当前仅保留 `build:solver`
- `.github/workflows/build.yml`：Windows CI；应保持与 `frontend/`、`solver/`、`wails.json` 和 `wails build` 链路一致

## 数据与代码模式

- 本仓库严格依赖 **3NF 扁平化结构**；优先用实体表 + 关系表，不要引入嵌套持久化结构。
- 当前前端表单中的组合字段只是 UI 视图：
  - 教师 `teaches` 实际落在 `teacher_courses`
  - 教师 `campus_ids` 实际落在 `teacher_campuses`
  - 课程 `place` 实际落在 `course_venues`
- `frontend/src/stores/data.js` 是高风险文件：这里既有自动保存，又有业务变更、副作用和课表聚合逻辑。
- `internal/backend/storage.go` 与 `internal/backend/import_export.go` 中允许存在最小旧数据兼容逻辑，但它们不能重新引入已删除功能的运行时依赖。

## 验证要求

本仓库当前没有自动化测试；不要主动新增测试框架或 CI 流水线，除非用户明确要求。

按改动范围选择最小验证：
- 仅文档改动：无需构建
- 前端页面 / 路由改动：`npm --prefix frontend run build`
- 求解器 / 数据模型改动：`npm run build:solver`
- Go / Wails / 打包改动：`go build ./...`、`wails build`
- 涉及真实交互流：`wails dev`

最低验证建议：
- 宿主调用层改动后：确认前端可加载并能调用后端
- 数据层改动后：确认加载 / 保存 / 提交 / 回滚 / 导入导出可走通
- solver 链路改动后：确认 solver 可构建、可调用、结果可写回
- 关闭流程改动后：确认未保存退出确认仍可触发

## Git 习惯

- 当前提交历史采用 Conventional Commits 风格，例如：`fix: ...`、`refactor: ...`、`docs: ...`
- 对跨层迁移或清理，优先做**小而完整**的提交：一次收口一条主链路。

## 边界

### Always
- 先读再改，不要凭 README 猜实现。
- 遵循“最小充分彻底原则”：不做超出用户目标的顺手重构；但对已纳入范围的功能，必须跨层清理到位。
- 删除或替换功能时必须一路追到：页面 / store / 宿主适配层 / Go 方法 / 数据结构 / 构建链路 / README / AGENTS。
- 做与改动规模匹配的构建验证。
- 更新 `README.md` 与 `AGENTS.md`，不要让剩余文档继续描述已不存在的宿主链路。

### Ask First
- 删除**不在当前用户清单内**的其他跨层模块
- 变更双表 temp / commit / revert 核心机制本身
- 修改导入导出功能范围
- 对自动排课做超出“适配现有数据模型”以外的约束重写
- 改动锁文件或大规模依赖收口：`frontend/package-lock.json`、`go.sum`
- 删除或修改 `%APPDATA%\com.tsxb.course-scheduler\` 下的用户真实数据

### Never
- 不要自行决定额外丢弃功能。
- 不要只改 UI，不改宿主 / 后端 / 数据结构。
- 不要恢复已删除的登录 / RBAC / 审计 / 日志系统。
- 不要编辑生成目录：`frontend/dist/`、`frontend/wailsjs/`、`frontend/node_modules/`、`node_modules/`、`solver/.venv/`、`solver/build/`、`solver/dist/`、`build/bin/`
- 不要把 `teacher_campuses` 降级成布尔字段或字符串字段。
- 不要为了让构建通过而删除无关逻辑或跳过文档同步。
- 不要恢复任何用户已删除的目录或文件，除非用户明确要求。

## 完成前检查清单

- `frontend/src/host/desktop.js` 是否仍保持宿主调用面与前端页面一致
- `internal/backend/models.go`、`internal/backend/storage.go`、`internal/backend/import_export.go` 是否与 `AllData` 契约一致
- `internal/backend/storage.go`、`internal/backend/import_export.go` 是否仅在边界保留 `is_only_shahe` 兼容逻辑
- `solver/solver.py` 是否仍按 `teacher_campuses` 集合约束建模
- `solver/scripts/prepare-solver.js` 与 `solver/dist/` 是否仍与 Wails sidecar 链路一致
- `README.md`、`AGENTS.md` 是否与实际仓库结构和运行方式一致
