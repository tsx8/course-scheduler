const ISSUE_SEVERITY = {
    ERROR: 'error',
    WARNING: 'warning',
};

const UNKNOWN = {
    teacher: '未知教师',
    course: '未知课程',
    campus: '未知校区',
    venue: '未知场地',
    day: '未知日期',
    time: '未知时段'
};

const sortIds = (ids) => [...new Set(ids.filter(Boolean))].sort();

const byId = (items = []) => new Map(items.map(item => [item.id, item]));

const nameOf = (map, id, fallback) => map.get(id)?.name || map.get(id)?.value || fallback;


const relationKey = (...parts) => parts.map(part => part || '').join('::');

const createIssue = ({
    category,
    severity = ISSUE_SEVERITY.ERROR,
    message,
    scheduleIds = [],
    teacherId = null,
    courseId = null,
    campusId = null,
    venueId = null,
    dayId = null,
    timeId = null,
    targetRoute = null,
    focus = null
}) => ({
    id: [category, teacherId, courseId, campusId, venueId, dayId, timeId, ...sortIds(scheduleIds)]
        .filter(value => value !== null && value !== undefined && value !== '')
        .join('|'),
    severity,
    category,
    message,
    schedule_ids: sortIds(scheduleIds),
    teacher_id: teacherId,
    course_id: courseId,
    campus_id: campusId,
    venue_id: venueId,
    day_id: dayId,
    time_id: timeId,
    target_route: targetRoute || (teacherId ? 'TeacherTimetable' : (campusId ? 'CampusTimetable' : null)),
    focus
});

const scheduleFocus = (schedule) => ({
    schedule_id: schedule.id,
    teacher_id: schedule.teacher_id,
    course_id: schedule.course_id,
    campus_id: schedule.campus_id,
    venue_id: schedule.venue_id,
    day_id: schedule.day_id,
    time_id: schedule.time_id
});

export function buildScheduleDiagnostics(data = {}) {
    const teachers = data.teachers || [];
    const courses = data.courses || [];
    const campuses = data.campuses || [];
    const venues = data.venues || [];
    const days = data.day || [];
    const times = data.time || [];
    const courseVenues = data.course_venues || [];
    const teacherCourses = data.teacher_courses || [];
    const teacherCampuses = data.teacher_campuses || [];
    const scheduledClasses = data.scheduled_classes || [];
    const teacherUnavailability = data.teacher_unavailability || [];
    const scheduleDensity = data.schedule_density || [];

    const teacherMap = byId(teachers);
    const courseMap = byId(courses);
    const campusMap = byId(campuses);
    const venueMap = byId(venues);
    const dayMap = byId(days);
    const timeMap = byId(times);

    const teacherCourseSet = new Set(teacherCourses.map(rel => relationKey(rel.teacher_id, rel.course_id)));
    const courseVenueSet = new Set(courseVenues.map(rel => relationKey(rel.course_id, rel.venue_id)));
    const teacherCampusSet = new Set(teacherCampuses.map(rel => relationKey(rel.teacher_id, rel.campus_id)));
    const teacherCampusCounts = teacherCampuses.reduce((counts, rel) => {
        counts.set(rel.teacher_id, (counts.get(rel.teacher_id) || 0) + 1);
        return counts;
    }, new Map());
    const unavailableSet = new Set(teacherUnavailability.map(rel => relationKey(rel.teacher_id, rel.day_id, rel.time_id)));
    const expectedCampusSlotCounts = new Map();
    scheduleDensity.forEach(density => {
        const count = Number(density.count || 0);
        expectedCampusSlotCounts.set(
            relationKey(density.campus_id, density.day_id, density.time_id),
            Math.max(0, Number.isFinite(count) ? count : 0)
        );
    });


    const issues = [];
    const activeSchedules = scheduledClasses.filter(schedule => schedule.is_staged !== true);


    const pushScheduleIssue = (schedule, category, severity, message) => {
        issues.push(createIssue({
            category,
            severity,
            message,
            scheduleIds: [schedule.id],
            teacherId: schedule.teacher_id,
            courseId: schedule.course_id,
            campusId: schedule.campus_id,
            venueId: schedule.venue_id,
            dayId: schedule.day_id,
            timeId: schedule.time_id,
            focus: scheduleFocus(schedule)
        }));
    };


    activeSchedules.forEach(schedule => {

        if (!teacherCourseSet.has(relationKey(schedule.teacher_id, schedule.course_id))) {
            pushScheduleIssue(
                schedule,
                'teacher_course_mismatch',
                ISSUE_SEVERITY.ERROR,
                `${nameOf(teacherMap, schedule.teacher_id, UNKNOWN.teacher)} 未配置可教授 ${nameOf(courseMap, schedule.course_id, UNKNOWN.course)}。`
            );
        }

        if ((teacherCampusCounts.get(schedule.teacher_id) || 0) > 0 && !teacherCampusSet.has(relationKey(schedule.teacher_id, schedule.campus_id))) {
            pushScheduleIssue(
                schedule,
                'teacher_campus_mismatch',
                ISSUE_SEVERITY.ERROR,
                `${nameOf(teacherMap, schedule.teacher_id, UNKNOWN.teacher)} 未配置可在 ${nameOf(campusMap, schedule.campus_id, UNKNOWN.campus)} 上课。`
            );
        }

        if (!courseVenueSet.has(relationKey(schedule.course_id, schedule.venue_id))) {
            pushScheduleIssue(
                schedule,
                'course_venue_mismatch',
                ISSUE_SEVERITY.ERROR,
                `${nameOf(courseMap, schedule.course_id, UNKNOWN.course)} 未配置可使用 ${nameOf(venueMap, schedule.venue_id, UNKNOWN.venue)}。`
            );
        }

        if (unavailableSet.has(relationKey(schedule.teacher_id, schedule.day_id, schedule.time_id))) {
            pushScheduleIssue(
                schedule,
                'teacher_unavailable_conflict',
                ISSUE_SEVERITY.ERROR,
                `${nameOf(teacherMap, schedule.teacher_id, UNKNOWN.teacher)} 在 ${nameOf(dayMap, schedule.day_id, UNKNOWN.day)} ${nameOf(timeMap, schedule.time_id, UNKNOWN.time)} 不可授课。`
            );
        }

        const venue = venueMap.get(schedule.venue_id);
        if (venue && Number(venue.capacity) <= 0) {
            pushScheduleIssue(
                schedule,
                'venue_capacity_warning',
                ISSUE_SEVERITY.WARNING,
                `${nameOf(venueMap, schedule.venue_id, UNKNOWN.venue)} 容量未设置或为 0。`
            );
        }
    });

    const byTeacherTime = new Map();
    const byTeacherDay = new Map();
    const byCampusSlot = new Map();
    const byVenueSlot = new Map();

    activeSchedules.forEach(schedule => {
        const teacherTimeKey = relationKey(schedule.teacher_id, schedule.day_id, schedule.time_id);
        const teacherDayKey = relationKey(schedule.teacher_id, schedule.day_id);
        const venueSlotKey = relationKey(schedule.venue_id, schedule.day_id, schedule.time_id);
        const campusSlotKey = relationKey(schedule.campus_id, schedule.day_id, schedule.time_id);


        if (!byTeacherTime.has(teacherTimeKey)) byTeacherTime.set(teacherTimeKey, []);
        if (!byTeacherDay.has(teacherDayKey)) byTeacherDay.set(teacherDayKey, []);
        if (!byVenueSlot.has(venueSlotKey)) byVenueSlot.set(venueSlotKey, []);
        if (!byCampusSlot.has(campusSlotKey)) byCampusSlot.set(campusSlotKey, []);

        byTeacherTime.get(teacherTimeKey).push(schedule);
        byTeacherDay.get(teacherDayKey).push(schedule);
        byCampusSlot.get(campusSlotKey).push(schedule);

        byVenueSlot.get(venueSlotKey).push(schedule);
    });

    byTeacherTime.forEach(schedules => {
        if (schedules.length <= 1) return;
        const first = schedules[0];
        issues.push(createIssue({
            category: 'teacher_time_conflict',
            severity: ISSUE_SEVERITY.ERROR,
            message: `${nameOf(teacherMap, first.teacher_id, UNKNOWN.teacher)} 在 ${nameOf(dayMap, first.day_id, UNKNOWN.day)} ${nameOf(timeMap, first.time_id, UNKNOWN.time)} 有 ${schedules.length} 节课冲突。`,
            scheduleIds: schedules.map(schedule => schedule.id),
            teacherId: first.teacher_id,
            dayId: first.day_id,
            timeId: first.time_id,
            focus: { teacher_id: first.teacher_id, day_id: first.day_id, time_id: first.time_id }
        }));
    });

    byTeacherDay.forEach(schedules => {
        const campusIds = sortIds(schedules.map(schedule => schedule.campus_id));
        if (campusIds.length <= 1) return;
        const first = schedules[0];
        issues.push(createIssue({
            category: 'teacher_day_campus_conflict',
            severity: ISSUE_SEVERITY.ERROR,
            message: `${nameOf(teacherMap, first.teacher_id, UNKNOWN.teacher)} 在 ${nameOf(dayMap, first.day_id, UNKNOWN.day)} 被安排到多个校区。`,
            scheduleIds: schedules.map(schedule => schedule.id),
            teacherId: first.teacher_id,
            dayId: first.day_id,
            focus: { teacher_id: first.teacher_id, day_id: first.day_id }
        }));
    });

    byCampusSlot.forEach(schedules => {
        const first = schedules[0];
        const expectedCount = expectedCampusSlotCounts.get(relationKey(first.campus_id, first.day_id, first.time_id)) || 0;
        if (schedules.length <= expectedCount) return;
        issues.push(createIssue({
            category: 'campus_density_overload',
            severity: ISSUE_SEVERITY.WARNING,
            message: `${nameOf(campusMap, first.campus_id, UNKNOWN.campus)} 在 ${nameOf(dayMap, first.day_id, UNKNOWN.day)} ${nameOf(timeMap, first.time_id, UNKNOWN.time)} 安排 ${schedules.length} 节，超过时段目标 ${expectedCount} 节。`,
            scheduleIds: schedules.map(schedule => schedule.id),
            campusId: first.campus_id,
            dayId: first.day_id,
            timeId: first.time_id,
            focus: { campus_id: first.campus_id, day_id: first.day_id, time_id: first.time_id }
        }));
    });

    byVenueSlot.forEach(schedules => {
        const first = schedules[0];
        const venue = venueMap.get(first.venue_id);
        const capacity = Number(venue?.capacity || 0);
        if (capacity <= 0 || schedules.length <= capacity) return;
        issues.push(createIssue({
            category: 'venue_capacity_warning',
            severity: ISSUE_SEVERITY.WARNING,
            message: `${nameOf(venueMap, first.venue_id, UNKNOWN.venue)} 在 ${nameOf(dayMap, first.day_id, UNKNOWN.day)} ${nameOf(timeMap, first.time_id, UNKNOWN.time)} 安排 ${schedules.length} 节，超过容量 ${capacity}。`,
            scheduleIds: schedules.map(schedule => schedule.id),
            campusId: first.campus_id,
            venueId: first.venue_id,
            dayId: first.day_id,
            timeId: first.time_id,
            focus: { campus_id: first.campus_id, venue_id: first.venue_id, day_id: first.day_id, time_id: first.time_id }
        }));
    });



    teachers.forEach(teacher => {
        const teacherSchedules = activeSchedules.filter(schedule => schedule.teacher_id === teacher.id);
        const totalHours = teacherSchedules.reduce((total, schedule) => {
            return total + Number(timeMap.get(schedule.time_id)?.corresponding_hours || 0);
        }, 0);
        const maxHours = Number(teacher.max_teaching_hours || 0);
        if (maxHours > 0 && totalHours > maxHours) {
            issues.push(createIssue({
                category: 'teacher_hours_warning',
                severity: ISSUE_SEVERITY.WARNING,
                message: `${teacher.name || UNKNOWN.teacher} 已安排 ${totalHours} 学时，超过最大授课学时 ${maxHours}。`,
                scheduleIds: teacherSchedules.map(schedule => schedule.id),
                teacherId: teacher.id,
                focus: { teacher_id: teacher.id }
            }));
        }

    });


    return issues.sort((left, right) => left.id.localeCompare(right.id));
}
