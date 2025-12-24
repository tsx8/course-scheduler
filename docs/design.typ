#import "./lib/lib.typ": *
#import "@preview/cetz:0.4.2"

#show: ori.with(
  title: "课设系统设计报告",
  author: "林子淇，邓少儒，琚长昊",
  subject: "数据库系统原理",
  semester: "2025 秋",
  maketitle: true,
  makeoutline: true,
  first-line-indent: auto,
)

= 系统需求分析

高校排课管理系统的核心目标是解决*复杂约束条件下的教学资源时空配置问题*。在学术管理语境下，资源配置不仅涉及*教师、课程、校区及场地*等基础元数据的静态维护，更包含排课过程中产生的*多维动态约束校验*。本系统需服务于两类核心角色：*排课员*作为全局调度者，需具备跨校区、跨时段的决策权限；*教师*作为资源使用者，仅获授权访问与其关联的特定业务数据视图。

为了支撑排课过程中高频次的方案试错与策略博弈，设计层面必须引入*数据隔离编辑机制*。这种机制要求系统在持久化层构建一个“逻辑沙箱”，允许排课指令在不干扰正式生效方案的前提下进行局部更新或全量回滚。此外，出于系统安全与操作不可抵赖性的考虑，所有针对数据状态的变动必须被实时捕捉并序列化为可追溯的*审计向量*。在约束模型方面，系统需同时处理*硬性物理约束*（如教师时空唯一性、场地容量阈值）与*软性管理目标*（如校区密度均衡、教学日集中度优化），这对底层关系模式的规范化程度提出了极高的要求。

== 数据流图

系统逻辑架构由数据指令解析、约束优化求解以及持久化状态机三个关键环节构成。排课员下达的行政配置指令作为原始数据流输入，首先触发数据管理模块的验证逻辑，在更新临时缓冲区的同时产生操作差异日志。自动排课求解器则作为独立的处理过程，从临时状态中提取约束向量，通过组合优化算法计算出排课候选解空间。最终，候选解在经过人工复核后，通过特定的原子提交指令完成从临时态（Temp）到主态（Main）的变迁，实现数据生命周期的状态转换。
\
\
#figure(
  cetz.canvas({
    import cetz.draw: *

    let mark-style = (end: ">", fill: black, scale: .5)

    rect((-7.5, 0.5), (-5.5, 1.5), name: "user", radius: 2pt)
    content("user", [排课员])

    circle((0, 1), radius: 1.2, name: "p1")
    content("p1", [P1\ 数据管理\ 与审计])

    circle((5, 1), radius: 1.2, name: "p2")
    content("p2", [P2\ 自动排课\ 求解器])

    line((-1.5, -2), (1.5, -2), name: "ds1_t")
    line((-1.5, -2.8), (1.5, -2.8), name: "ds1_b")
    content((0, -2.4), [D1 临时表 Temp])

    line((3.5, -2), (6.5, -2), name: "ds2_t")
    line((3.5, -2.8), (6.5, -2.8), name: "ds2_b")
    content((5, -2.4), [D2 主表 Main])

    line("user.east", "p1.west", mark: mark-style)
    content((-3.5, 1.3), [配置指令], size: 9pt)

    line("p1.south", (0, -2), mark: mark-style)
    content((1, -1), [写入/回滚], size: 9pt)

    line((1.2, 1.2), (3.8, 1.2), mark: mark-style)
    content((2.5, 1.5), [约束向量], size: 9pt)
    line((3.8, 0.8), (1.2, 0.8), mark: mark-style)
    content((2.5, 0.5), [排课候选解], size: 9pt)

    line((1.5, -2.4), (3.5, -2.4), mark: mark-style)
    content((2.5, -2.1), [原子提交], size: 9pt)

    bezier("p1.north", "user.north", (0, 3.5), (-6, 4), mark: mark-style)
    content((-3, 3.6), [审计反馈/视图渲染], size: 9pt)
  }),
  caption: [系统数据流图 (DFD)],
)

== 数据元素表

依据数据字典规范，系统核心数据元素定义如下表所示，这些元素构成了系统逻辑演绎的物理基础：

#figure(
  table(
    columns: (auto, auto, 1fr),
    [*数据元素名称*], [*数据类型*], [*语义说明*],
    [Teacher_ID], [UUID / TEXT], [教师实体唯一标识，作为授课资质与考勤业务的主码],
    [Max_Hours], [INTEGER], [教师在单一教学周期（周）内允许授课的总时长硬约束阈值],
    [Is_Only_Shahe], [BOOLEAN], [物理空间限制标志，定义教师是否被限定在沙河校区执教],
    [Course_Place], [RELATION], [课程被授权进行的校区与场地的多维可行空间映射],
    [Schedule_ID], [UUID / TEXT], [排课元组的主码，唯一确定时间、地点、课程的组合],
    [Density_Target], [INTEGER], [用户定义的特定校区在特定时段的期望班级承载数量],
  ),
  caption: [核心数据元素定义],
)

= 数据库概念模式设计

在概念设计阶段，我们将复杂的排课业务抽象为一系列实体及其相互联系。系统模型围绕四个核心实体展开：教师、课程、场地与校区。这些实体通过授课、准入、属于以及具有多向绑定性质的“排课”联系发生关联。为了精准刻画现实教学逻辑，模型将教师与课程定义为多对多（M:N）的授课资质关系，反映了高校教师多学科授课的特征；课程与场地亦呈现多对多（M:N）的准入关系，以适应不同课程对多媒体或专业实验室的需求；而场地与校区之间则受到严格的地理从属约束，呈现出确定性的多对一（N:1）关系。

\

#figure(
  cetz.canvas({
    import cetz.draw: *
    
    set-style(
      rect: (stroke: 0.8pt),
      line: (stroke: 0.8pt),
      circle: (stroke: 0.8pt)
    )
    
    rect((-6, 4), (-4, 5), name: "teacher")
    content("teacher", [教师])
    
    rect((4, 4), (6, 5), name: "course")
    content("course", [课程])
    
    rect((4, -2), (6, -1), name: "venue")
    content("venue", [场地])
    
    rect((-6, -2), (-4, -1), name: "campus")
    content("campus", [校区])
    
    polygon((0, 4.5), 4, name: "rel_teaches")
    content("rel_teaches", [授课])
    
    polygon((5, 1.5), 4, name: "rel_access")
    content("rel_access", [准入])

    polygon((0, -1.5), 4, name: "rel_belong")
    content("rel_belong", [属于])
    
    polygon((0, 1.5), 4, name: "rel_schedule")
    content("rel_schedule", [排课])

    line("teacher", "rel_teaches")
    content((-2.5, 4.8), [M], size: 8pt)
    line("rel_teaches", "course")
    content((2.5, 4.8), [N], size: 8pt)
    
    line("course", "rel_access")
    content((5.3, 3), [M], size: 8pt)
    line("rel_access", "venue")
    content((5.3, 0), [N], size: 8pt)
    
    line("venue", "rel_belong")
    content((2.5, -1.8), [N], size: 8pt)
    line("rel_belong", "campus")
    content((-2.5, -1.8), [1], size: 8pt)
    
    line("teacher", "rel_schedule")
    line("course", "rel_schedule")
    line("venue", "rel_schedule")
    content((-2, 3), [1], size: 8pt)
    content((2, 3), [1], size: 8pt)
    content((2, 0), [1], size: 8pt)

    let attr(pos, label, target, name: none) = {
      circle(pos, radius: (0.8, 0.4), name: name)
      content(pos, label, size: 8pt)
      line(name, target, stroke: (thickness: 0.5pt))
    }

    attr((-7, 6), [#underline[教师ID]], "teacher.north-west", name: "t_id")
    attr((-5, 6.5), [姓名], "teacher.north", name: "t_name")
    attr((-3, 6), [最大学时], "teacher.north-east", name: "t_hours")

    attr((5, 6.5), [#underline[课程ID]], "course.north", name: "c_id")
    attr((7, 6), [课程名], "course.north-east", name: "c_name")

    attr((7, -3), [#underline[场地ID]], "venue.south-east", name: "v_id")
    attr((5, -3.5), [名称], "venue.south", name: "v_name")
    attr((3, -3), [容量], "venue.south-west", name: "v_cap")

    attr((-7, -3), [#underline[校区ID]], "campus.south-west", name: "cp_id")
    attr((-5, -3.5), [校区名], "campus.south", name: "cp_name")
  }),
  caption: [系统总体概念模式 (E-R 图)],
)

\

在该 E-R 模型中，“排课”不仅是一次实体间的简单关联，它更是高度结构化的复合联系，隐含地绑定了时间维度中的“时段”与“工作日”属性。所有的实体属性均围绕主码展开，不存在多值属性或复杂的复合项。通过引入校区实体，模型确立了场地的层级化管理逻辑，为后续在逻辑设计中实施校区级、场地级的级联约束奠定了理论框架。

= 数据库逻辑模式设计

依据概念模式，我们进一步将系统演绎为关系模型。为了根除关系运算中可能出现的更新异常与数据冗余，本系统严格遵循第三范式（3NF）的规范化准则进行模式设计与验证。

== 关系模式定义

系统定义的生产环境核心关系模式集合如下。我们采用关系代数记法表示，其中下划线表征主码，星号（\*）标记表征引用完整性约束下的外码。

#set par(justify: false)
+ *地理维*：Campus(#underline[id], name)；Venue(#underline[id], #emph[campus_id]\*, name, capacity)
+ *元数据维*：Course(#underline[id], name)；Teacher(#underline[id], name, max_hours, is_only_shahe)
+ *时间维*：TimeSlot(#underline[id], value, hours)；Day(#underline[id], value)
+ *资质关联关系*（M:N）：TeacherCourse(#underline[#emph[teacher_id]\*, #emph[course_id]\*])
+ *场地准入关系*（M:N）：CourseVenue(#underline[#emph[course_id]\*, #emph[venue_id]\*])
+ *排课核心关系*：ScheduledClass(#underline[id], #emph[teacher_id]\*, #emph[course_id]\*, #emph[day_id]\*, #emph[time_id]\*, #emph[campus_id]\*, #emph[venue_id]\*)
+ *约束控制关系*：ScheduleDensity(#underline[#emph[campus_id]\*, #emph[day_id]\*, #emph[time_id]\*], count)
#set par(justify: true)

== 规范化论证与 3NF 验证

系统逻辑模式的设计核心在于通过投影分解消除非平凡的函数依赖冲突，以下是针对规范化等级的理论验证：

首先，系统确保了所有关系的*第一范式（1NF）*合规性，每个字段均存储不可分割的原子值，如 UUID 标识符或离散的学时整数。其次，针对*第二范式（2NF）*的要求，系统彻底消除了非主属性对码的部分函数依赖。以 `ScheduledClass` 为例，尽管其主码为全局唯一的 `id`，但即便在隐含的复合码（教师+时段+工作日）视阈下，该关系的地点与课程属性亦是完全函数依赖于该组合，不存在仅依赖于“教师”或“时段”的非主属性。

最后，通过对*第三范式（3NF）*的严格对标，系统消除了非主属性间的传递函数依赖。在早期的初步构思中，曾考虑在场地关系中冗余存储校区名称，但这会导致 `Venue_ID -> Campus_ID -> Campus_Name` 的传递依赖链。通过将 `Campus` 独立成表并在 `Venue` 中仅保留 `campus_id` 外码，系统确保了非主属性 `name` 和 `capacity` 仅直接依赖于其主码，从而从根本上规避了校区更名时可能引发的数据一致性灾难。

== 物理隔离：双表镜像事务架构

为了在物理层实现需求分析中定义的“乐观编辑”与“安全隔离”，逻辑模式为每个核心业务关系构建了对应的 `_temp` 镜像关系。在这一架构下，所有的 `INSERT`、`UPDATE` 指令在初始阶段均定向至临时缓冲区，形成了一个与生产环境隔离的“待发布视图”。

这种设计在工程逻辑上是对数据库原子性（Atomicity）与隔离性（Isolation）的延伸。当排课员完成一系列复杂的关联修改后，系统通过一个原子性的事务序列，将临时态的元组集与主态进行同步。这种基于逻辑模式镜像的物理优化，不仅保证了在 3NF 高度分解架构下的关联查询性能，更为系统提供了近乎零成本的“撤销”能力，有效应对了排课决策中的不确定性。

== 完整性与级联约束策略

系统的完整性控制体系建立在严格的参照完整性基础之上。通过显式启用数据库引擎的外键检查功能，并为所有外码引用的 `DELETE` 操作配置 `CASCADE` 策略，系统建立起了一套自修复的数据网络。例如，当一个校区实体被行政注销时，其关联的场地、该场地的课程准入记录乃至历史排课轨迹，都将沿着关系链路被自动、原子地清理，从而在逻辑层杜绝了孤儿记录与悬挂指针的生成，确保了 3NF 模型在动态操作环境下的结构完整性。
