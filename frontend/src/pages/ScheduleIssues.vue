<script setup>
import { computed, h, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useDataStore } from '../stores/data';
import {
    NButton,
    NCard,
    NDataTable,
    NEmpty,
    NFlex,
    NIcon,
    NH2,
    NSelect,
    NSpace
} from 'naive-ui';
import { LocateOutline as LocateIcon } from '@vicons/ionicons5';
import ScheduleIssueTag, { getIssueSeverityMeta } from '../components/ScheduleIssueTag.vue';

const dataStore = useDataStore();
const router = useRouter();

const selectedSeverity = ref(null);
const selectedTeacherId = ref(null);
const selectedCampusId = ref(null);
const selectedDayId = ref(null);
const selectedTimeId = ref(null);

const severityOptions = [
    { label: '硬冲突', value: 'hard' },
    { label: '风险', value: 'warning' },
];

const issueSeverityOrder = {
    hard: 0,
    warning: 1,
};

const allIssues = computed(() => {
    return Array.isArray(dataStore.scheduleIssues) ? dataStore.scheduleIssues : [];
});

const teacherOptions = computed(() => dataStore.teachers.map(teacher => ({
    label: teacher.name,
    value: teacher.id
})));

const campusOptions = computed(() => dataStore.campuses.map(campus => ({
    label: campus.name,
    value: campus.id
})));

const dayOptions = computed(() => dataStore.day.map(day => ({
    label: day.value,
    value: day.id
})));

const timeOptions = computed(() => dataStore.time.map(time => ({
    label: time.value,
    value: time.id
})));

const lookup = computed(() => ({
    teachers: new Map(dataStore.teachers.map(teacher => [teacher.id, teacher])),
    courses: new Map(dataStore.courses.map(course => [course.id, course])),
    campuses: new Map(dataStore.campuses.map(campus => [campus.id, campus])),
    venues: new Map(dataStore.venues.map(venue => [venue.id, venue])),
    days: new Map(dataStore.day.map(day => [day.id, day])),
    times: new Map(dataStore.time.map(time => [time.id, time]))
}));

const nameOrMissing = (map, id, missingLabel) => {
    if (!id) return '-';
    return map.get(id)?.name || map.get(id)?.value || `${missingLabel}已删除`;
};

const issueMatchesFilter = (issue) => {
    const severityGroup = getIssueSeverityMeta(issue.severity).group;
    return (!selectedSeverity.value || severityGroup === selectedSeverity.value)
        && (!selectedTeacherId.value || issue.teacher_id === selectedTeacherId.value)
        && (!selectedCampusId.value || issue.campus_id === selectedCampusId.value)
        && (!selectedDayId.value || issue.day_id === selectedDayId.value)
        && (!selectedTimeId.value || issue.time_id === selectedTimeId.value);
};

const sortedIssues = computed(() => {
    return allIssues.value
        .filter(issueMatchesFilter)
        .slice()
        .sort((a, b) => {
            const aMeta = getIssueSeverityMeta(a.severity);
            const bMeta = getIssueSeverityMeta(b.severity);
            const severityDiff = issueSeverityOrder[aMeta.group] - issueSeverityOrder[bMeta.group];
            if (severityDiff !== 0) return severityDiff;

            const dayDiff = dataStore.day.findIndex(day => day.id === a.day_id) - dataStore.day.findIndex(day => day.id === b.day_id);
            if (dayDiff !== 0) return dayDiff;

            const timeDiff = dataStore.time.findIndex(time => time.id === a.time_id) - dataStore.time.findIndex(time => time.id === b.time_id);
            if (timeDiff !== 0) return timeDiff;

            return (a.message || '').localeCompare(b.message || '', 'zh-Hans-CN');
        });
});

const severityStats = computed(() => {
    const initial = {
        hard: 0,
        warning: 0,
    };

    return allIssues.value.reduce((stats, issue) => {
        const group = getIssueSeverityMeta(issue.severity).group;
        stats[group] += 1;
        return stats;
    }, initial);
});
const emptyDescription = computed(() => {
    return allIssues.value.length === 0 ? '未发现排课问题' : '当前筛选条件下没有排课问题';
});


const buildFocus = (issue) => {
    return issue.focus || {
        schedule_ids: Array.isArray(issue.schedule_ids) ? issue.schedule_ids : [],
        teacher_id: issue.teacher_id,
        course_id: issue.course_id,
        campus_id: issue.campus_id,
        venue_id: issue.venue_id,
        day_id: issue.day_id,
        time_id: issue.time_id
    };
};

const locateIssue = (issue) => {
    const focus = buildFocus(issue);

    if (typeof dataStore.setScheduleFocus === 'function') {
        dataStore.setScheduleFocus(focus);
    }

    if (focus.teacher_id) {
        dataStore.selectedTeacherIdForTeacherView = focus.teacher_id;
    }
    if (focus.campus_id) {
        dataStore.selectedCampusIdForCampusView = focus.campus_id;
    }
    if (focus.venue_id) {
        dataStore.selectedVenueIdsForCampusView = [focus.venue_id];
    } else if (focus.campus_id) {
        dataStore.selectedVenueIdsForCampusView = [];
    }
    if (focus.course_id) {
        dataStore.selectedCourseIdsForCampusView = [focus.course_id];
    } else if (focus.campus_id) {
        dataStore.selectedCourseIdsForCampusView = [];
    }

    router.push({ name: focus.teacher_id ? 'TeacherTimetable' : 'CampusTimetable' });
};

const columns = computed(() => [
    {
        title: '级别',
        key: 'severity',
        width: 90,
        sorter: (a, b) => issueSeverityOrder[getIssueSeverityMeta(a.severity).group] - issueSeverityOrder[getIssueSeverityMeta(b.severity).group],
        render(row) {
            return h(ScheduleIssueTag, { severity: row.severity });
        }
    },
    {
        title: '问题',
        key: 'message',
        minWidth: 260,
        sorter: (a, b) => (a.message || '').localeCompare(b.message || '', 'zh-Hans-CN')
    },
    {
        title: '教师',
        key: 'teacher_id',
        width: 140,
        render(row) {
            return nameOrMissing(lookup.value.teachers, row.teacher_id, '教师');
        }
    },
    {
        title: '课程',
        key: 'course_id',
        width: 140,
        render(row) {
            return nameOrMissing(lookup.value.courses, row.course_id, '课程');
        }
    },
    {
        title: '校区/场地',
        key: 'campus_id',
        width: 180,
        render(row) {
            const campus = nameOrMissing(lookup.value.campuses, row.campus_id, '校区');
            const venue = nameOrMissing(lookup.value.venues, row.venue_id, '场地');
            return row.venue_id ? `${campus} / ${venue}` : campus;
        }
    },
    {
        title: '时间',
        key: 'day_id',
        width: 150,
        render(row) {
            const day = nameOrMissing(lookup.value.days, row.day_id, '日期');
            const time = nameOrMissing(lookup.value.times, row.time_id, '时间');
            if (day === '-' && time === '-') return '-';
            return `${day} ${time}`;
        }
    },
    {
        title: '定位',
        key: 'actions',
        width: 100,
        align: 'center',
        render(row) {
            return h(NButton, {
                size: 'small',
                type: 'primary',
                secondary: true,
                onClick: () => locateIssue(row)
            }, {
                icon: () => h(NIcon, null, { default: () => h(LocateIcon) }),
                default: () => '定位'
            });
        }
    }
]);
</script>

<template>
    <section class="issues-page">
        <header class="issues-page__header">
            <n-flex justify="space-between" align="center" :wrap="false">
                <n-h2 style="margin: 0; white-space: nowrap; flex-shrink: 0;">问题检查</n-h2>
                <div class="issues-page__stats" aria-label="问题统计">
                    <span>冲突: <strong>{{ severityStats.hard }}</strong></span>
                    <span>风险: <strong>{{ severityStats.warning }}</strong></span>
                </div>
            </n-flex>
        </header>

        <main class="issues-page__content">
            <n-space vertical size="large">
                <n-card size="small">
                    <n-flex align="center">
                        <n-select v-model:value="selectedSeverity" placeholder="级别" :options="severityOptions" clearable
                            style="width: 120px" />
                        <n-select v-model:value="selectedTeacherId" placeholder="教师" :options="teacherOptions" clearable
                            filterable style="width: 160px" />
                        <n-select v-model:value="selectedCampusId" placeholder="校区" :options="campusOptions" clearable
                            filterable style="width: 160px" />
                        <n-select v-model:value="selectedDayId" placeholder="日期" :options="dayOptions" clearable
                            style="width: 140px" />
                        <n-select v-model:value="selectedTimeId" placeholder="时间" :options="timeOptions" clearable
                            style="width: 160px" />
                    </n-flex>
                </n-card>

                <n-data-table v-if="sortedIssues.length > 0" :columns="columns" :data="sortedIssues" :row-key="row => row.id"
                    :bordered="true" :single-line="false" :pagination="{ pageSize: 12 }" style="width: 100%;" />
                <n-empty v-else :description="emptyDescription" size="huge" style="padding: 64px 0;" />
            </n-space>
        </main>
    </section>
</template>

<style scoped>
.issues-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
}

.issues-page__header {
    flex: 0 0 auto;
    padding: 12px 24px;
    border-bottom: 1px solid #efeff5;
}

.issues-page__stats {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: baseline;
    gap: 16px;
    color: #606266;
    white-space: nowrap;
}

.issues-page__stats strong {
    color: #303133;
    font-size: 18px;
    line-height: 1;
}

.issues-page__content {
    flex: 1 1 auto;
    min-height: 0;
    padding: 24px;
    overflow: auto;
}
</style>