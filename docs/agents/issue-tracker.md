# Issue Tracker

本仓库的 issues 和 PRD 默认发布到 GitHub Issues：

- Remote: `https://github.com/tsx8/course-scheduler.git`
- Tool: `gh`

## Conventions

- 创建 issue: `gh issue create --title "..." --body "..."`
- 查看 issue: `gh issue view <number> --comments`
- 列出 issue: `gh issue list --state open --json number,title,body,labels,comments`
- 评论 issue: `gh issue comment <number> --body "..."`
- 增删 labels: `gh issue edit <number> --add-label "..."` / `gh issue edit <number> --remove-label "..."`
- 关闭 issue: `gh issue close <number> --comment "..."`

在仓库根目录执行 `gh` 命令，让 GitHub CLI 从 `git remote -v` 推断目标仓库。发布 PRD、拆分 issues 或 triage 前，先确认用户确实希望写入 GitHub。
