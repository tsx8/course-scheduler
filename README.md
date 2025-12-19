# Course Scheduler / 课程排课系统

一个基于 Tauri 的智能课程排课桌面应用，使用约束求解器自动生成课程时间表。

## ✨ 特性

- 🎯 **智能排课**: 基于 Google OR-Tools CP-SAT 求解器的约束优化算法
- 🏫 **多校区支持**: 支持跨校区教学安排和场地管理
- 👨‍🏫 **教师管理**: 灵活的教师课程分配和时间约束设置
- 📅 **可视化时间表**: 直观的教师和校区课表展示
- 💾 **临时保存机制**: 支持编辑预览和一键提交/回滚
- 🚀 **原生性能**: 基于 Tauri 框架，体积小、速度快、资源占用低

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
│   │   └── file_handler.rs # 文件操作命令
│   └── tauri.conf.json      # Tauri 配置
├── solver/                  # Python 排课求解器
│   ├── solver.py            # OR-Tools 约束模型
│   └── solver.spec          # PyInstaller 打包配置
└── scripts/
    └── prepare-solver.js    # 求解器构建脚本
```

## 🔧 核心机制

### 数据持久化：乐观临时-提交模式

所有编辑操作自动保存到 `data.tmp.json` 临时文件，用户可以：
- ✅ 安全地实验不同的排课方案
- ✅ 随时回滚到上次提交的状态
- ✅ 明确控制何时永久保存更改

```
用户编辑 → 自动保存到 tmp → 用户确认 → 提交到 data.json
                    ↓
                 可随时撤销
```

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
)
```

### 调试求解器

```bash
cd solver
# 使用 uv 运行（自动管理依赖）
uv run python solver.py input.json output.json

# 或使用虚拟环境
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
- `solver/solver.py` (Python DataManager)
- `src/stores/data.js` (Vue 响应式状态)

## 📦 构建与部署

### Windows
```bash
npm run tauri build
# 输出: src-tauri/target/release/bundle/msi/
```

## 🔮 路线图

- [ ] **PostgreSQL 迁移**: 替换 JSON 文件存储为关系数据库
- [ ] **多学期支持**: 管理不同学期的排课数据
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
