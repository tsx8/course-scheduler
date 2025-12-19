# Course Scheduler / 课程排课系统

一个基于 Tauri 的智能课程排课桌面应用，使用约束求解器自动生成课程时间表。

## ✨ 特性

- 🎯 **智能排课**: 基于 Google OR-Tools CP-SAT 求解器的约束优化算法
- 🏫 **多校区支持**: 支持跨校区教学安排和场地管理
- 👨‍🏫 **教师管理**: 灵活的教师课程分配和时间约束设置
- 📅 **可视化时间表**: 直观的教师和校区课表展示
- 💾 **临时保存机制**: 支持编辑预览和一键提交/回滚
- 🗄️ **SQLite 数据库**: 嵌入式数据库,零配置,自动备份
- 📤 **导入导出**: 支持 JSON 和 SQLite 数据库文件的导入导出
- 📝 **日志系统**: 自动记录应用运行日志,支持日期轮转和自动清理
- 🚀 **原生性能**: 基于 Tauri 框架,体积小、速度快、资源占用低

## 🏗️ 技术架构

### 前端
- **Vue 3** - 渐进式 JavaScript 框架
- **Vite** - 下一代前端构建工具
- **Naive UI** - Vue 3 组件库
- **Pinia** - Vue 状态管理
- **Vue Router** - 官方路由管理器

### 后端
- **Rust** - 系统级编程语言
- **Tauri** - 构建跨平台桌面应用的框架
- **SQLite** - 嵌入式关系数据库（通过 rusqlite 集成）
- **Tracing** - 结构化日志系统,支持异步写入和日志轮转

### 排课引擎
- **Python 3.11+** - 脚本语言
- **OR-Tools** - Google 的运筹学工具包
- **PyInstaller** - 打包为独立可执行文件

## 📋 系统要求

### 开发环境
- **Node.js** 18+ 
- **Rust** 1.70+ (安装 via [rustup](https://rustup.rs/))
- **Python** 3.11+ 
- **uv** - Python 包管理器 (`pip install uv`)

### 运行环境
仅需下载编译后的可执行文件，无需安装任何依赖。

## 🚀 快速开始

### 1. 克隆项目
```bash
git clone https://github.com/tsx8/course-scheduler.git
cd course-scheduler
```

### 2. 安装依赖
```bash
npm install
uv sync
```

### 3. 启动开发服务器
```bash
npm run tauri dev
```

### 4. 构建生产版本
```bash
npm run tauri build
```

生成的安装包位于 `src-tauri/target/release/bundle/` 目录。

## 📖 使用指南

### 基本工作流程

1. **设置校区和场地**
   - 在"场地管理"页面添加校区
   - 为每个校区添加教学场地（教室）
   - 设置场地容量

2. **创建课程**
   - 在"课程管理"页面添加课程
   - 指定课程可用的校区和场地

3. **配置教师**
   - 在"教师管理"页面添加教师信息
   - 分配教师负责的课程
   - 设置教学时数上限
   - 标记不可用时间段

4. **自动排课**
   - 点击"自动排课"按钮
   - 系统运行约束求解器生成时间表
   - 结果自动保存到临时文件

5. **查看和调整**
   - 在"教师课表"查看个人时间表
   - 在"校区课表"查看整体安排
   - 手动调整时间（可选）

6. **提交或回滚**
   - 满意后点击"提交更改"保存
   - 不满意点击"撤销更改"恢复到上次保存状态

### 约束规则

#### 硬约束（必须满足）
- ✅ 教师同一时间只能在一个地点上课
- ✅ 场地同一时间只能容纳一门课程
- ✅ 教师总课时不超过设定上限
- ✅ 教师不在标记为"不可用"的时间段排课
- ✅ 仅沙河校区的教师只在沙河上课
- ✅ 非沙河限定教师必须在两个校区都有课

#### 软约束（尽量优化）
- 📊 最小化教师单课日（只有一节课的工作日）
- 📊 最小化课程间隙（连续时段之间的空档）
- 📊 最小化零散工作日（减少非连续工作日）
- 📊 最大化课程开设数量
- 📊 均衡课程在不同校区的分布

## 🗂️ 项目结构

```
course-scheduler/
├── src/                      # Vue 前端源码
│   ├── pages/               # 页面组件
│   │   ├── CourseManagement.vue      # 课程管理
│   │   ├── TeacherManagement.vue     # 教师管理
│   │   ├── VenueManagement.vue       # 场地管理
│   │   ├── TeacherTimetable.vue      # 教师课表
│   │   └── CampusTimetable.vue       # 校区课表
│   ├── stores/              # Pinia 状态管理
│   │   └── data.js          # 核心数据存储
│   ├── components/          # 可复用组件
│   └── layouts/             # 布局组件
├── src-tauri/               # Rust 后端源码
│   ├── src/
│   │   ├── main.rs          # Tauri 主入口
│   │   ├── models.rs        # 数据模型定义
│   │   ├── db_handler.rs    # SQLite 数据库操作
│   │   ├── import_export.rs # 导入导出功能
│   │   ├── single_instance.rs # 单实例运行控制
│   │   └── lib.rs           # 库入口
│   ├── schema.sql           # 数据库表结构定义
│   └── tauri.conf.json      # Tauri 配置
├── solver/                  # Python 排课求解器
│   ├── solver.py            # OR-Tools 约束模型
│   └── solver.spec          # PyInstaller 打包配置
└── scripts/
    └── prepare-solver.js    # 求解器构建脚本
```

## 🔧 核心机制

### 数据持久化：SQLite 双表模式

应用使用 SQLite 数据库实现乐观临时-提交模式：
- 💾 **主表**: 存储已提交的永久数据
- 🔄 **临时表** (`_temp` 后缀): 存储编辑中的工作状态
- ⚡ **自动保存**: 每次编辑后 100ms 自动保存到临时表
- ✅ **提交**: 将临时表内容复制到主表
- ↩️ **回滚**: 清空临时表，恢复到上次提交状态

```
用户编辑 → 自动保存到 temp 表 → 用户确认 → 提交到主表
                    ↓
                 可随时撤销
```Roaming\com.tsxb.course-scheduler\course_scheduler.db`
- 日志文件: `%APPDATA%\Roaming\com.tsxb.course-scheduler\logs\course-scheduler-YYYY.MM.DD.log`
- 旧版 JSON 文件在首次启动时自动迁移,备份为 `data.json.backup`

**备份建议**: 定期复制 `course_scheduler.db` 文件到安全位置

### 导入导出功能

支持多种格式的数据导入导出:
- 📥 **JSON 导入**: 将 JSON 格式的课程数据导入到临时表（需提交后生效）
- 📥 **数据库导入**: 从其他 SQLite 数据库文件导入数据
- 📤 **JSON 导出**: 将当前数据导出为 JSON 文件（便于分享和备份）
- 📤 **数据库导出**: 将数据库完整复制到指定位置（包含主表和临时表）

**注意**: 导入操作会先写入临时表,用户可以预览后再决定是否提交

### 日志系统

应用集成了完善的日志系统:
- 📝 **日期轮转**: 每天生成新的日志文件（格式: `course-scheduler-YYYY.MM.DD.log`）
- 🧹 **自动清理**: 保留最近 3 天的日志,删除 30 天前的旧日志
- 🔄 **智能恢复**: 日志文件被删除后会自动重新创建
- 📊 **日志级别**: 支持通过 `RUST_LOG` 环境变量控制日志详细程度_scheduler.db`
- 旧版 JSON 文件在首次启动时自动迁移为 `data.json.backup`

**备份建议**: 定期复制 `course_scheduler.db` 文件到安全位置

### 排课求解器集成

1. **输入**: 当前所有数据（教师、课程、场地、约束）
2. **处理**: Python 求解器运行 CP-SAT 算法（默认 60 秒超时）
3. **输出**: 优化后的课程安排
4. **应用**: 结果写入临时文件，前端自动刷新

## 🛠️ 开发指南

### 修改约束权重

编辑 `solver/solver.py` 第 422 行的目标函数权重：

```python
model.Minimize(
    penalty_single_class_days * 10 +    # 单课日惩罚
    penalty_gaps * 5 +                   # 课程间隙惩罚
    penalty_scattered_workdays * 3 +     # 零散工作日惩罚
    penalty_course_openings * 100 +      # 课程开设惩罚
    penalty_campus_imbalance * 2 +       # 校区不均衡惩罚
    penalty_hour_imbalance * 1           # 课时不均衡惩罚
)相关模块（如 `db_handler.rs`、`import_export.rs`）定义函数
2. 使用 `#[tauri::command]` 宏标注
3. 在 `main.rs` 的 `invoke_handler!` 注册
4. 前端通过 `invoke('command_name')` 调用

### 数据模型修改

⚠️ **关键**: `AllData` 结构在三处必须同步更新：
- `src-tauri/src/models.rs` (Rust 类型)
- `src-tauri/schema.sql` (SQLite 表结构)
- `solver/solver.py` (Python DataManager)
- `src/stores/data.js` (Vue 响应式状态)

### 数据库 Schema 管理

数据库表结构定义在 `src-tauri/schema.sql`:
- **主表**: 直接表名（如 `teachers`、`courses`）
- **临时表**: 表名后缀 `_temp`（如 `teachers_temp`、`courses_temp`）
- **外键约束**: 启用 `PRAGMA foreign_keys = ON` 确保数据一致性
- **级联删除**: 使用 `ON DELETE CASCADE` 自动清理关联数据

修改表结构后需要:
1. 更新 `schema.sql` 文件
2. 同步更新 `models.rs` 中的 Rust 类型
3. 更新 `db_handler.rs` 中的数据库操作代码
uv venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
uv pip install -r pyproject.toml
python solver.py input.json output.json
```

### 添加新的 Tauri 命令

1. 在 `src-tauri/src/main.rs` 或 `file_handler.rs` 定义函数
2. 使用 `#[tauri::command]` 宏标注
3. 在 `main.rs` 的 `invoke_handler!` 注册
4. 前端通过 `invoke('command_name')` 调用

### 数据模型修改

⚠️ **关键**: `AllData` 结构在三处必须同步更新：
- `src-tauri/src/models.rs` (Rust 类型)
- [x] **导入导出功能**: ✅ 已完成 - 支持 JSON 和数据库文件的导入导出
- [x] **日志系统**: ✅ 已完成 - 结构化日志,支持日期轮转和自动清理
- [x] **单实例运行**: ✅ 已完成 - 防止同时打开多个应用实例
- [ ] **冲突检测**: 实时显示排课冲突
- [ ] **Excel/PDF 导出
## 📦 构建与部署

### Windows
```bash
npm run tauri build
# 输出: src-tauri/target/release/bundle/msi/
```

## 🔮 路线图

- [x] **SQLite 数据库**: ✅ 已完成 - 替换 JSON 文件为嵌入式数据库
- [ ] **冲突检测**: 实时显示排课冲突
- [ ] **导出功能**: 支持导出 Excel/PDF 格式课表
- [ ] **统计分析**: 课程分布、教师工作量可视化

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 💡 技术支持

遇到问题？
- 📖 查看 [Tauri 文档](https://tauri.app)
- 📖 查看 [OR-Tools 文档](https://developers.google.com/optimization)
- 🐛 提交 [Issue](https://github.com/tsx8/course-scheduler/issues)

## 🙏 致谢

- [Tauri](https://tauri.app) - 跨平台桌面应用框架
- [Vue 3](https://vuejs.org) - 渐进式前端框架
- [Naive UI](https://www.naiveui.com) - Vue 3 组件库
- [OR-Tools](https://developers.google.com/optimization) - Google 运筹学工具包

---

⭐ 如果这个项目对你有帮助，请给个 Star！
