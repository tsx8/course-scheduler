# Domain Docs

本仓库按单上下文项目处理：根目录的 `CONTEXT.md` 承载领域词汇，`docs/adr/` 承载架构决策记录。两者当前可以按需创建，不需要为了普通代码修改提前补空文件。

## Before Exploring

- 如果存在 `CONTEXT.md`，先读取它，用其中词汇描述排课领域概念。
- 如果存在 `docs/adr/`，读取与当前任务相关的 ADR，避免重复提出已否决或已定稿的方案。
- 如果二者不存在，继续按 `AGENTS.md`、`README.md` 和代码事实探索；只有在 `grill-with-docs` 或架构讨论中真正沉淀了术语/决策时再创建。

## Vocabulary Discipline

输出 issue、PRD、诊断假设、重构建议和测试名称时，优先沿用本项目既有术语，例如：

- 桌面排课应用
- Wails 宿主
- 本地后端
- Python solver sidecar
- SQLite 主表 / `_temp` 表
- 编辑态、保存、提交、回滚
- 教师上课校区
- 校区总课表、教师个人课表

如果新术语会影响后续开发沟通，用 `grill-with-docs` 明确含义后写入 `CONTEXT.md`。
