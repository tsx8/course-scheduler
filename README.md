# Course Scheduler / 课程排课系统

一个基于 Tauri 的智能课程排课桌面应用，使用约束求解器自动生成课程时间表。

## ✨ 特性

- 🎯 **智能排课**: 基于 Google OR-Tools CP-SAT 求解器的约束优化算法
- 🏫 **多校区支持**: 支持跨校区教学安排和场地管理
- 👨‍🏫 **教师管理**: 灵活的教师课程分配和时间约束设置
- 📅 **可视化时间表**: 直观的教师和校区课表展示
- 💾 **临时保存机制**: 双表 SQLite 架构实现乐观临时-提交模式
- 🗄️ **规范化数据库**: 符合第三范式的数据库设计，消除数据冗余
- 🔒 **RBAC 权限系统**: 基于角色的访问控制，支持 Scheduler 和 Teacher 两种角色
- 📋 **审计日志**: 完整的操作审计追踪，记录所有数据变更历史，支持详细的字段级差异对比
- 👤 **用户管理**: 完善的用户账号管理，支持密码重置和角色分配
- 🗃️ **导入导出**: 支持 JSON 和 SQLite 数据库文件的导入导出
- 📝 **日志系统**: 自动记录应用运行日志,支持日期轮转和自动清理
- 🚀 **原生性能**: 基于 Tauri 2 框架,体积小、速度快、资源占用低
- 🪟 **Windows 专属**: 针对 Windows 10+ 优化的桌面体验

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
- **bcrypt** - 密码哈希加密，保障用户账号安全
- **Tracing** - 结构化日志系统,支持异步写入和日志轮转

### 排课引擎
- **Python 3.11+** - 脚本语言
- **OR-Tools** - Google 的运筹学工具包
- **PyInstaller** - 打包为独立可执行文件

## 📋 系统要求

### 开发环境
- **操作系统**: Windows 10+ (项目仅支持 Windows 平台)
- **Node.js** 18+ 
- **Rust** 1.70+ (通过 [rustup](https://rustup.rs/) 安装)
- **Python** 3.11+ 
- **uv** - 快速 Python 包管理器 (`pip install uv`)

### 运行环境
仅需下载编译后的可执行文件，无需安装任何依赖。

**平台说明**: 尽管 Tauri 2 支持跨平台，本项目专为 Windows 优化，不提供 macOS/Linux 支持。

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

### 3. 构建求解器
```bash
npm run build:solver
```

### 4. 启动开发服务器
```bash
npm run tauri dev
```

### 5. 构建生产版本
```bash
npm run tauri build
```

生成的安装包位于 `src-tauri/target/release/bundle/` 目录。

## 📖 使用指南

### 身份认证与权限

**首次使用**:
- 默认管理员账号: `admin`
- 默认密码: `123456`
- 首次登录后建议立即修改默认密码

**角色说明**:
- **Scheduler（排课员）**: 拥有完整系统权限，可管理所有数据、用户和查看审计日志
- **Teacher（教师）**: 仅可查看个人课表和导出个人数据

### 基本工作流程

1. **登录系统**
   - 使用用户名和密码登录
   - 系统根据角色自动跳转到对应页面

2. **用户管理**（仅 Scheduler）
   - 在"用户管理"页面创建新用户
   - 为用户分配角色（Scheduler 或 Teacher）
   - 可选：为 Teacher 角色关联对应的教师信息
   - 支持重置用户密码和删除账号

3. **设置校区和场地**（仅 Scheduler）
   - 在"场地管理"页面添加校区
   - 为每个校区添加教学场地（教室）
   - 设置场地容量
   - 设置排课密度

4. **创建课程**（仅 Scheduler）
   - 在"课程管理"页面添加课程
   - 指定课程可用的校区和场地

5. **配置教师**（仅 Scheduler）
   - 在"教师管理"页面添加教师信息
   - 分配教师负责的课程
   - 设置教学时数上限
   - 标记不可用时间段

6. **自动排课**（仅 Scheduler）
   - 点击"自动排课"按钮
   - 系统运行约束求解器生成时间表
   - 结果自动保存到临时表

7. **查看和调整**
   - **Scheduler**: 在"校区总课表"查看整体安排，可手动调整
   - **Teacher**: 在"教师个人课表"查看自己的时间表

8. **提交或回滚**（仅 Scheduler）
   - 满意后点击"提交更改"保存
   - 不满意点击"撤销更改"恢复到上次保存状态

9. **审计追踪**（仅 Scheduler）
   - 在"审计日志"页面查看所有操作记录
   - 支持按时间、操作类型、用户等条件筛选
   - 追踪数据变更历史和责任人

### 约束规则

#### 硬约束（必须满足）
- ✅ 教师同一时间只能在一个地点上课
- ✅ 场地同一时间只能容纳一门课程
- ✅ 教师总课时不超过设定上限
- ✅ 教师不在标记为"不可用"的时间段排课
- ✅ 仅沙河校区的教师只在沙河上课（`is_only_shahe` 标志）
- ✅ 非沙河限定教师必须在两个校区都有课（两校区规则）
- ✅ 教师每天只能在一个校区上课（场地-校区映射）
- ✅ 已排满 4 天及以上的教师不再增加新的工作日

#### 软约束（尽量优化）
- 📊 尽量接近每个校区每个时段设定的目标班级数量。
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
│   │   ├── Login.vue                 # 登录页面
│   │   ├── CourseManagement.vue      # 课程管理
│   │   ├── TeacherManagement.vue     # 教师管理
│   │   ├── VenueManagement.vue       # 场地管理
│   │   ├── TeacherTimetable.vue      # 教师课表
│   │   ├── CampusTimetable.vue       # 校区课表
│   │   ├── UserManagement.vue        # 用户管理（RBAC）
│   │   ├── AuditLogs.vue             # 审计日志查看
│   │   └── Settings.vue              # 设置和导入导出
│   ├── stores/              # Pinia 状态管理
│   │   ├── data.js          # 核心数据存储 + 自动保存
│   │   └── auth.js          # 认证和会话管理
│   ├── components/          # 可复用组件
│   │   └── ScheduleCell.vue          # 课表单元格
│   ├── layouts/             # 布局组件
│   │   └── MainLayout.vue            # 主布局 + 自定义标题栏
│   └── router/              # 路由配置
│       └── index.js         # 路由定义 + RBAC 守卫
├── src-tauri/               # Rust 后端源码
│   ├── src/
│   │   ├── main.rs          # Tauri 主入口 + 求解器调用
│   │   ├── models.rs        # AllData 数据模型定义
│   │   ├── db_handler.rs    # SQLite 数据库操作 + 双表逻辑
│   │   ├── auth.rs          # 密码哈希和验证（bcrypt）
│   │   ├── audit.rs         # 审计日志记录
│   │   ├── import_export.rs # 导入导出功能
│   │   ├── single_instance.rs # 单实例运行控制
│   │   └── lib.rs           # 库入口
│   ├── schema.sql           # 数据库表结构定义（主表 + 临时表 + RBAC 表）
│   └── tauri.conf.json      # Tauri 配置 + 求解器 sidecar
├── solver/                  # Python 排课求解器
│   ├── solver.py            # OR-Tools CP-SAT 约束模型
│   └── solver.spec          # PyInstaller 打包配置
├── scripts/
│   └── prepare-solver.js    # 求解器构建脚本
└── .github/
    └── copilot-instructions.md # AI 编码代理详细说明
```

## 🔧 核心机制

### 数据持久化：SQLite 双表模式（Optimistic Temp-to-Commit）

应用使用 SQLite 数据库实现乐观临时-提交模式，支持安全的试错和撤销：
- 💾 **主表**: 存储已提交的永久数据（如 `teachers`, `courses`, `campuses`, `audit_logs`）
- 🔄 **临时表** (`_temp` 后缀): 存储编辑中的工作状态（如 `teachers_temp`, `courses_temp`, `audit_logs_temp`）
- ⚡ **自动保存**: 编辑后 100ms 防抖保存到临时表（`save_temp_data` 命令）
- ✅ **提交**: 将临时表内容复制到主表（`commit_data` 命令）
- ↩️ **回滚**: 清空临时表，恢复到上次提交状态（`clear_temp_data` 命令）
- 🔒 **事务安全**: 利用 SQLite ACID 特性保证数据完整性
- 🎯 **求解器集成**: 排课结果写入临时表，需提交后生效
- 📋 **审计同步**: 审计日志同样遵循双表模式，未提交的操作日志存储在临时表中，提交后同步至主表。

```
用户编辑 → 防抖 100ms → 自动保存到 temp 表 → 用户确认 → 提交到主表
                              ↓
                          可随时撤销（清空 temp 表）
```

**Windows 数据位置**:
- 数据库文件: `%APPDATA%\com.tsxb.course-scheduler\course_scheduler.db`
- 日志文件: `%APPDATA%\com.tsxb.course-scheduler\logs\course-scheduler-YYYY.MM.DD.log`

**备份建议**: 定期复制 `course_scheduler.db` 文件到安全位置

**智能数据同步**: 当课程的场地发生变化时，`data.js` 中的 Pinia store 会自动更新所有相关教师的已排课程（智能替换逻辑，防止孤立引用）。

## 🏛️ 数据库设计：第三范式 (3NF)

### 设计原则

本项目严格遵循第三范式 (3NF) 数据库设计规范，确保数据一致性和系统可维护性。

**3NF 核心要求**:
1. ✅ 所有属性都是原子性的（不可再分）
2. ✅ 所有非主键属性完全依赖于主键
3. ✅ 所有非主键属性之间不存在传递依赖

### 规范化实践

#### 1. **消除数据冗余**

**course_venues 关系表**（课程-场地多对多关系）:
```sql
-- 只存储关系本身，不冗余存储 campus_id
CREATE TABLE course_venues (
    course_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (course_id, venue_id),
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues(id) ON DELETE CASCADE
);
```

**设计优势**:
- ✅ campus_id 只在 venues 表存储一次（单一数据源）
- ✅ 修改 venue 所属校区只需更新一处
- ✅ 不存在数据不一致的可能性

#### 2. **扁平化数据模型**

前端数据模型与数据库结构保持一致，采用扁平化关系存储:

```javascript
// 扁平化存储
const campuses = ref([
  { id: 'campus-1', name: '沙河校区' }
]);

const venues = ref([
  { id: 'v1', campus_id: 'campus-1', name: '教室101', capacity: 30 }
]);

// 通过计算属性获取关联数据
const getCampusVenues = (campusId) => {
  return venues.value.filter(v => v.campus_id === campusId);
};
```

**优势**:
- ✅ 数据更新简单（不需要维护嵌套结构）
- ✅ 内存占用更低（无重复对象）
- ✅ 序列化/反序列化效率高

#### 3. **外键约束与级联删除**

启用严格的数据库约束，确保引用完整性:

```sql
PRAGMA foreign_keys = ON;

-- 级联删除示例
CREATE TABLE course_venues (
    course_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues(id) ON DELETE CASCADE
);
```

**自动维护关系**:
- ✅ 删除 campus → 自动删除关联的 venues 和 course_venues
- ✅ 删除 teacher → 自动删除 teacher_courses 和 scheduled_classes
- ✅ 删除 course → 自动删除 course_venues 和 scheduled_classes
- ✅ 事务保证操作原子性（全部成功或全部回滚）

#### 4. **索引优化**

为关联查询创建索引，提升查询性能:

```sql
CREATE INDEX idx_course_venues_course ON course_venues(course_id);
CREATE INDEX idx_course_venues_venue ON course_venues(venue_id);
CREATE INDEX idx_venues_campus ON venues(campus_id);
CREATE INDEX idx_teacher_courses_teacher ON teacher_courses(teacher_id);
CREATE INDEX idx_teacher_courses_course ON teacher_courses(course_id);
```

### 开发规范

**修改数据库时必须保持 3NF 合规性**:

❌ **错误示例**（违反 3NF）:
```sql
-- 在 scheduled_classes 中冗余存储 teacher_name
CREATE TABLE scheduled_classes (
    teacher_id TEXT,
    teacher_name TEXT,  -- 违反 3NF！
    ...
);
```

✅ **正确示例**（符合 3NF）:
```sql
-- 只存储外键，通过 JOIN 获取属性
CREATE TABLE scheduled_classes (
    teacher_id TEXT,
    FOREIGN KEY (teacher_id) REFERENCES teachers(id)
);

-- 查询时关联获取
SELECT sc.*, t.name as teacher_name
FROM scheduled_classes sc
JOIN teachers t ON sc.teacher_id = t.id;
```

**数据建模检查清单**:
1. ❌ 不要在多个表中存储相同的非键属性
2. ❌ 不要创建传递依赖（A → B → C，其中 C 依赖于 B 而非 A）
3. ✅ 确保每个非键属性直接依赖于主键
4. ✅ 使用外键约束强制引用完整性
5. ✅ 为高频关联查询创建索引

### 导入导出功能

支持多种格式的数据导入导出，所有导入操作写入**临时表**:
- 📥 **JSON 导入**: 解析 `AllData` (3NF结构) → 写入临时表 → 用户预览 → 提交
- 📥 **数据库导入**: 打开外部 `.db` 文件 → 验证 schema → 复制到临时表
- 📤 **JSON 导出**: 查询当前状态 → 序列化为 `AllData` JSON → 写入文件
- 📤 **数据库导出**: 复制整个 `.db` 文件（包含主表和临时表）

**注意**: 导入操作会先写入临时表,用户可以预览后再决定是否提交

**数据完整性**: SQLite 启用外键约束（`PRAGMA foreign_keys = ON`），使用 `ON DELETE CASCADE` 自动清理关联数据。

### 日志系统

应用集成了完善的日志系统:
- 📝 **日期轮转**: 每天生成新的日志文件（格式: `course-scheduler-YYYY.MM.DD.log`）
- 🧹 **自动清理**: 保留最近 3 天的日志,删除 30 天前的旧日志
- 🔄 **智能恢复**: 日志文件被删除后会自动重新创建
- 📊 **日志级别**: 支持通过 `RUST_LOG` 环境变量控制日志详细程度

### 排课求解器集成

求解器作为独立进程（sidecar）与 Rust 后端通信：

1. **输入**: 用户点击"自动排课" → Rust 序列化当前数据为 JSON (3NF格式) → 传递给求解器
2. **处理**: Python 求解器运行 CP-SAT 算法（默认 60 秒超时，`solver/solver.py`）
3. **输出**: 优化后的课程安排（JSON 格式，包含 `scheduled_classes` 列表）
4. **应用**: Rust 解析输出 → 写入临时表 → 前端 Pinia store 自动刷新

**约束模型** (`solver/solver.py`):
- **DataManager**: 预处理扁平化输入数据，建立索引映射
- **硬约束**: 教师/场地冲突，课时上限，校区规则
- **软约束**: 最小化单课日、课程间隙、零散工作日，以及排课密度
- **目标函数**: 多权重优化，平衡 6 个因素

**PyInstaller 打包**: `solver.spec` 手动包含 OR-Tools 二进制文件（`_pywrapcp.pyd`, `.libs`），输出为 `solver-x86_64-pc-windows-msvc.exe`。

## 🛠️ 开发指南

### ⚠️ 重要约束

- **平台限制**: 仅支持 Windows 10+，不要添加 macOS/Linux 兼容代码
- **测试策略**: 仅手动测试，不要创建自动化测试套件或 CI/CD 测试管道

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

然后重新构建求解器：

```bash
cd solver
uv run pyinstaller solver.spec
cd ..
node scripts/prepare-solver.js
```

### 添加新的 Tauri 命令

1. 在相关模块（如 `db_handler.rs`、`import_export.rs`）定义函数
2. 使用 `#[tauri::command]` 宏标注
3. 在 `main.rs` 的 `invoke_handler!` 注册
4. 前端通过 `invoke('command_name')` 调用

### 数据模型修改

⚠️ **关键（四处同步更新）**: `AllData` 结构是前端、后端、数据库、求解器之间的契约，修改必须同步：

1. **Rust 模型**: `src-tauri/src/models.rs` - 定义 Serde 序列化结构
2. **SQLite Schema**: `src-tauri/schema.sql` - 数据库表结构（主表 + `_temp` 表）
3. **Python DataManager**: `solver/solver.py` - 求解器输入数据类
4. **Frontend Store**: `src/stores/data.js` - Vue 响应式状态定义

**同步修改流程**:
1. 更新 `models.rs` 中的结构体字段
2. 更新 `schema.sql` 中的表定义（主表 + 临时表）
3. 更新 `db_handler.rs` 中的序列化/反序列化逻辑
4. 更新 `data.js` 中的响应式 refs
5. 同步 `solver.py` 中的 DataManager

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

### 独立测试求解器

```bash
cd solver
uv venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
uv pip install -r pyproject.toml
python solver.py input.json output.json
```
## 📦 构建与部署

### Windows
```bash
npm run tauri build
# 输出: src-tauri/target/release/bundle/msi/
```

## 🔮 路线图

已完成:
- [x] **SQLite 数据库**: ✅ 双表模式（主表 + 临时表）
- [x] **RBAC 权限系统**: ✅ 基于角色的访问控制
- [x] **审计日志基础**: ✅ 操作记录和追踪，支持字段级差异对比
- [x] **日志系统**: ✅ 日期轮转 + 自动清理
- [x] **自定义标题栏**: ✅ 无边框窗口 + 自定义最小化/最大化/关闭按钮
- [x] **数据格式统一**: ✅ 全面移除旧版 JSON 兼容代码，系统完全基于 3NF 架构

下一步计划:
- [ ] **会话管理优化**: 移除进程锁机制，改由登录系统控制单账户单会话，提升多用户并发能力
- [ ] **导出功能完善**: 优化数据导出流程，支持选择性导出和多格式输出（CSV、Excel）
- [ ] **冲突检测**: 实时显示排课冲突和约束违反提示
- [ ] **统计分析**: 课程分布、教师工作量数据可视化

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
