# Course Scheduler / 课程排课系统

一个基于 `Vue 3 + Wails 2 + Go + SQLite + Python OR-Tools` 的 Windows 桌面排课应用。

## 特性

- 智能排课：基于 OR-Tools CP-SAT 求解器生成课表
- 双表机制：SQLite 主表 / `_temp` 表支持自动保存、提交、回滚
- 3NF 建模：核心数据以实体表 + 关系表组织，避免冗余
- 多校区排课：支持校区、场地、课程、教师的关联约束
- 教师上课校区：教师可从现有 `campuses` 中多选可上课校区，默认全选
- 可视化课表：提供校区总课表与教师个人课表视图
- 导入导出：支持 JSON 与 SQLite 数据导入导出
- Windows 桌面体验：Wails 原生宿主、自定义标题栏、sidecar 求解器、可打包发布

## 技术栈

### 前端
- `Vue 3`
- `Vite`
- `Naive UI`
- `Pinia`
- `Vue Router`

### 桌面宿主与本地后端
- `Wails 2`
- `Go`
- `SQLite`（`modernc.org/sqlite`）

### 求解器
- `Python 3.11+`
- `OR-Tools`
- `PyInstaller`

## 系统要求

### 开发环境
- Windows 10+
- Node.js 18+
- Go 1.25+
- Python 3.11+
- `uv`
- `wails` CLI 2.10+

### 运行环境
- Windows 10+
- WebView2 Runtime

## 快速开始

```bash
npm --prefix frontend install
uv --directory solver sync
uv --directory solver run pyinstaller solver.spec
go build ./...
wails dev
```

生产构建：

```bash
wails build -nsis
```

构建产物输出目录：`build/bin/`。CI 上传 `*-installer.exe` 安装包；安装包默认安装到当前用户目录 `%LOCALAPPDATA%\Programs\course-scheduler`。

## 使用流程

1. 在“场地/校区管理”中维护校区与场地
2. 在“课程信息管理”中维护课程及允许场地
3. 在“教师信息管理”中维护教师、可教课程、最大课时、上课校区
4. 在“设置”中维护时间段、工作日，并执行导入导出
5. 点击“自动排课”调用求解器生成课表
6. 在“校区总课表”或“教师个人课表”中查看与手动调整
7. 通过“保存更改”提交，或“撤销所有更改”回滚

## 约束规则

### 硬约束
- 教师同一时间最多上一门课
- 场地同一时间不得超容量占用
- 教师总课时不得超过上限
- 教师不可用时段不得排课
- 教师只能在其“上课校区”集合内上课
- 教师同一天最多出现在一个校区
- 已排满 4 天及以上的教师不再增加新的工作日

### 软约束
- 尽量贴近校区时段排课密度目标
- 减少教师单课日
- 减少教师日内课程间隙
- 减少零散工作日
- 提高教师-课程-校区组合的开设覆盖度

## 教师“上课校区”模型

教师可上课校区采用 3NF 关系表建模：

- 主表：`teacher_campuses`
- 临时表：`teacher_campuses_temp`
- 前端表单：从 `campuses` 中多选
- 默认值：全选当前所有校区
- 求解器行为：仅在教师允许的校区集合内为其创建候选排课变量

旧数据中的 `is_only_shahe` 仅作为兼容迁移来源：

- `true` → 映射为“沙河校区”集合
- `false` → 映射为当前全部校区集合

当前兼容边界位于：

- `internal/backend/storage.go`
- `internal/backend/import_export.go`

## 项目结构

```text
course-scheduler/
├── build/
│   ├── appicon.png
│   └── windows/
│       ├── icon.ico
│       ├── info.json
│       ├── wails.exe.manifest
│       └── installer/
│           └── project.nsi
├── frontend/
│   ├── index.html
│   ├── package.json
│   ├── package-lock.json
│   ├── vite.config.js
│   ├── src/
│   │   ├── components/
│   │   ├── host/
│   │   │   └── desktop.js
│   │   ├── layouts/
│   │   ├── pages/
│   │   ├── router/
│   │   ├── stores/
│   │   ├── App.vue
│   │   └── main.js
│   ├── dist/      # 生成物
│   └── wailsjs/   # 生成物
├── internal/
│   └── backend/
│       ├── app.go
│       ├── import_export.go
│       ├── models.go
│       ├── solver.go
│       └── storage.go
├── solver/
│   ├── .python-version
│   ├── pyproject.toml
│   ├── uv.lock
│   ├── solver.py
│   ├── solver.spec
│   ├── build/     # 生成物
│   └── dist/      # 生成物；sidecar 单一来源
├── main.go
├── wails.json
└── AGENTS.md
```

## 核心机制

### 双表机制

应用使用 SQLite 双表机制管理编辑态与已提交态：

- 主表：持久生效数据
- `_temp` 表：编辑中的工作副本
- 自动保存：前端变更防抖写入 `_temp`
- 提交：将 `_temp` 完整覆盖回主表
- 回滚：清空 `_temp`，恢复到上次提交状态

```text
用户编辑 → 自动保存到 _temp → 人工确认 → 提交到主表
                     ↓
                   可撤销
```

Windows 数据目录：

- 数据库：`%APPDATA%\com.tsxb.course-scheduler\course_scheduler.db`
- 求解器临时输入/输出：`%APPDATA%\com.tsxb.course-scheduler\solver_input.tmp.json`、`solver_output.tmp.json`
- 解包后的求解器：`%APPDATA%\com.tsxb.course-scheduler\solver.exe`

### 3NF 设计

本项目遵循第三范式：

- `teacher_courses`：教师-课程关系
- `course_venues`：课程-场地关系
- `teacher_campuses`：教师-校区关系

不要把多值关系压成布尔字段、逗号拼接字符串或 JSON 持久化字段。

### 排课求解器

求解器作为 Python sidecar 运行：

1. Go 读取当前数据并序列化为 JSON
2. Wails 宿主调用 `solver.exe`
3. Python 构建 CP-SAT 模型并求解
4. 结果写回 `_temp` 表，等待用户提交

构建 sidecar：

```bash
uv --directory solver run pyinstaller solver.spec
```

该命令会在 `solver/` 子项目中使用 `uv` 环境执行 PyInstaller，把 `solver/solver.py` 构建到 `solver/dist/solver.exe`，并由 Go/Wails 直接以该目录作为 sidecar 单一来源。`wails build` 与 `wails build -nsis` 都会通过 `wails.json` 的 `preBuildHooks` 自动执行同一构建。

## 开发说明

### 常用命令

```bash
npm --prefix frontend run build
uv --directory solver run pyinstaller solver.spec
go build ./...
wails dev
wails build -nsis
```

### CI

Windows CI 位于 `.github/workflows/build.yml`，当前按 Wails 目录结构执行：

```bash
npm --prefix frontend ci
uv --directory solver sync --frozen
uv --directory solver run pyinstaller solver.spec
npm --prefix frontend run build
go build ./...
wails build -nsis
```

### 数据模型修改时需要同步更新

修改 `AllData` 契约时，至少同步：

1. `internal/backend/models.go`
2. `internal/backend/storage.go`
3. `internal/backend/import_export.go`
4. `frontend/src/stores/data.js`
5. `solver/solver.py`

### 手动验证建议

- 前端改动后运行 `npm --prefix frontend run build`
- 求解器 / 数据模型改动后运行 `uv --directory solver run pyinstaller solver.spec`
- Go / Wails 改动后运行 `go build ./...`
- 打包验证运行 `wails build -nsis`

## 许可证

MIT
