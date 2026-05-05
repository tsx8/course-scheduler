<script setup>
import { computed, h, ref, watch } from 'vue';
import { useDataStore } from '../stores/data';
import { NSelect, NDataTable, NSpace, NH2, NEmpty, NTag, NFlex, NButton, NIcon } from 'naive-ui';
import { DownloadOutline as DownloadIcon } from '@vicons/ionicons5';
import ScheduleCell from '../components/ScheduleCell.vue';
import ScheduleDetailDrawer from '../components/ScheduleDetailDrawer.vue';

const dataStore = useDataStore();
const detailDrawer = ref({
    show: false,
    title: '排课详情',
    issues: [],
});


const teacherOptions = computed(() => dataStore.teacherOptions);
const timeSlots = computed(() => dataStore.time);
const days = computed(() => dataStore.day);

const selectedTeacherId = computed({
    get: () => dataStore.selectedTeacherIdForTeacherView,
    set: (val) => {
        dataStore.selectedTeacherIdForTeacherView = val;
    }
});

const selectedTeacher = computed(() => {
    if (!selectedTeacherId.value) return null;
    return dataStore.teachers.find(t => t.id === selectedTeacherId.value);
});

const teacherSchedules = computed(() => {
    if (!selectedTeacherId.value) return [];
    return dataStore.scheduledClassesByTeacher(selectedTeacherId.value);
});

const scheduledHours = computed(() => {
    if (!selectedTeacherId.value) return 0;

    const timeHoursMap = new Map(dataStore.time.map(t => [t.id, t.corresponding_hours]));
    return teacherSchedules.value.reduce((total, schedule) => {
        const hours = timeHoursMap.get(schedule.time_id) || 0;
        return total + hours;
    }, 0);
});

const maxHours = computed(() => {
    return selectedTeacher.value ? selectedTeacher.value.max_teaching_hours : 0;
});

const isOverLimit = computed(() => {
    return selectedTeacher.value ? scheduledHours.value > maxHours.value : false;
});
const teacherIssues = computed(() => {
    if (!selectedTeacherId.value) return [];
    return dataStore.issuesByTeacher.get(selectedTeacherId.value) || [];
});

const teacherIssueCounts = computed(() => teacherIssues.value.reduce((counts, issue) => {
    counts[issue.severity] = (counts[issue.severity] || 0) + 1;
    return counts;
}, { error: 0, warning: 0 }));

const teacherWorkdayCount = computed(() => new Set(teacherSchedules.value.map(schedule => schedule.day_id).filter(Boolean)).size);
const teacherCampusCount = computed(() => new Set(teacherSchedules.value.map(schedule => schedule.campus_id).filter(Boolean)).size);

const teacherSummaryItems = computed(() => [
    { label: '课时', value: `${scheduledHours.value}/${maxHours.value}`, type: isOverLimit.value ? 'error' : null },
    { label: '上课', value: `${teacherWorkdayCount.value} 天` },
    { label: '校区', value: `${teacherCampusCount.value} 校区` },
    {
        label: '异常',
        value: `${teacherIssueCounts.value.error} 冲突 / ${teacherIssueCounts.value.warning} 风险`,
        type: teacherIssueCounts.value.error > 0 ? 'error' : (teacherIssueCounts.value.warning > 0 ? 'warning' : null),
    },
]);

const openTeacherDetail = () => {
    if (!selectedTeacher.value) return;
    detailDrawer.value = {
        show: true,
        title: `${selectedTeacher.value.name} 排课详情`,
        issues: teacherIssues.value,
    };
};


const handleDetailLocate = (issue) => {
    dataStore.setScheduleFocus(issue.focus || { type: 'teacher', teacher_id: selectedTeacherId.value });
    detailDrawer.value.show = false;
};


const tableData = computed(() => {
    if (!selectedTeacherId.value) return [];
    return timeSlots.value.map(time => {
        const row = { key: time.id, time_slot: time.value };
        days.value.forEach(day => {
            row[day.id] = { day_id: day.id, time_id: time.id };
        });
        return row;
    });
});

const columns = computed(() => {
    if (!selectedTeacherId.value) return [];

    const dayColumns = days.value.map(day => ({
        title: day.value,
        key: day.id,
        align: 'center',
        render(row) {
            return h(ScheduleCell, {
                teacherId: selectedTeacherId.value,
                dayId: row[day.id].day_id,
                timeId: row[day.id].time_id,
            });
        }
    }));

    return [
        {
            title: '时间',
            key: 'time_slot',
            width: '10%',
            align: 'center',
            fixed: 'left',
            resizable: true,
            render(row) {
                return h('div', { style: 'font-weight: bold;' }, row.time_slot);
            }
        },
        ...dayColumns
    ];
});

const exportCSV = () => {
    if (!selectedTeacherId.value || !selectedTeacher.value) return;

    const teacher = selectedTeacher.value;
    const teacherSchedules = dataStore.scheduledClassesByTeacher(selectedTeacherId.value);
    const escapeCsvCell = (value) => `"${String(value).replace(/"/g, '""')}"`;
    const formatSchedule = (schedule) => {
        const course = dataStore.courses.find(c => c.id === schedule.course_id);
        const campus = dataStore.campuses.find(c => c.id === schedule.campus_id);
        const venue = dataStore.venues.find(v => v.id === schedule.venue_id);
        return `${course?.name || ''} - ${campus?.name || ''} - ${venue?.name || ''}`;
    };

    const rows = [
        ['教师课表', teacher.name],
        ['时间段', ...days.value.map(day => day.value)],
        ...timeSlots.value.map(time => [
            time.value,
            ...days.value.map(day => teacherSchedules
                .filter(s => s.day_id === day.id && s.time_id === time.id)
                .map(formatSchedule)
                .join('\n'))
        ])
    ];

    const csv = '\uFEFF' + rows.map(row => row.map(escapeCsvCell).join(',')).join('\n');

    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);
    link.setAttribute('href', url);
    link.setAttribute('download', `${teacher.name}_课表.csv`);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
};

watch(() => dataStore.teachers, (newTeachers) => {
    if (selectedTeacherId.value && !newTeachers.find(t => t.id === selectedTeacherId.value)) {
        selectedTeacherId.value = null;
    }
}, { deep: true });
</script>

<template>
    <section class="timetable-page">
        <header class="timetable-page__header">
            <n-flex justify="space-between" align="center" :wrap="false">
                <n-h2 style="margin: 0; white-space: nowrap; flex-shrink: 0;">教师个人课表</n-h2>
                <n-space :wrap="false" align="center" style="flex-shrink: 1; overflow: hidden;">
                    <n-select v-model:value="selectedTeacherId" placeholder="请选择教师"
                        :options="teacherOptions" filterable clearable style="width: 200px" />
                    <n-button :disabled="!selectedTeacherId" type="primary" @click="exportCSV">
                        <template #icon>
                            <n-icon>
                                <DownloadIcon />
                            </n-icon>
                        </template>
                        导出 CSV
                    </n-button>
                </n-space>
            </n-flex>
        </header>

        <main class="timetable-page__content">
            <div v-if="selectedTeacher" class="summary-bar" role="button" tabindex="0" @click="openTeacherDetail" @keyup.enter="openTeacherDetail" @keyup.space="openTeacherDetail">
                <div v-for="item in teacherSummaryItems" :key="item.label" class="summary-bar__item" :class="item.type ? `summary-bar__item--${item.type}` : null">
                    <span>{{ item.label }}</span>
                    <strong>{{ item.value }}</strong>
                </div>
                <n-tag v-if="isOverLimit" class="summary-bar__tag" type="error" size="small">课时超限</n-tag>
                <n-button class="summary-bar__action" tertiary size="small" type="primary" @click.stop="openTeacherDetail">详情</n-button>
            </div>

            <div v-if="selectedTeacherId" class="timetable-scroll">
                <n-data-table :columns="columns" :data="tableData" :bordered="true"
                    :single-line="false" style="width: 100%;" />
            </div>
            <n-flex v-else justify="center" align="center" class="timetable-empty">
                <n-empty description="请先选择一位教师以管理其课表" size="huge" />
            </n-flex>
        </main>

        <ScheduleDetailDrawer
            v-model:show="detailDrawer.show"
            :title="detailDrawer.title"
            :issues="detailDrawer.issues"
            @locate="handleDetailLocate"
        />
    </section>
</template>

<style scoped>
.timetable-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    position: relative;
    overflow: hidden;
}

.timetable-page__header {
    flex: 0 0 auto;
    padding: 12px 24px;
    border-bottom: 1px solid #efeff5;
}

.timetable-page__content {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    padding: 24px;
    overflow: hidden;
}
.timetable-scroll {
    flex: 0 1 auto;
    min-height: 0;
    max-height: 100%;
    overflow: auto;
}

.timetable-empty {
    flex: 1 1 auto;
    min-height: 0;
}

.summary-bar {
    display: flex;
    align-items: center;
    align-self: flex-start;
    flex: 0 0 auto;
    flex-wrap: wrap;
    gap: 6px;
    max-width: 100%;
    margin-bottom: 12px;
    padding: 8px;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    background: #f8fafc;
    cursor: pointer;
}

.summary-bar:focus-visible {
    outline: 2px solid #18a058;
    outline-offset: 2px;
}

.summary-bar__item {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
    padding: 5px 8px;
    border: 1px solid #e5e7eb;
    border-radius: 999px;
    background: #fff;
}

.summary-bar__item span {
    display: block;
    color: #606266;
    font-size: 12px;
    line-height: 1;
}

.summary-bar__item strong {
    display: block;
    color: #303133;
    font-size: 14px;
    line-height: 1;
}

.summary-bar__item--error {
    border-color: #f2b8b5;
    background: #fff6f6;
}

.summary-bar__item--warning {
    border-color: #f3d19e;
    background: #fff8ee;
}

.summary-bar__item--error strong {
    color: #d03050;
}

.summary-bar__item--warning strong {
    color: #d46b08;
}
.summary-bar__tag,
.summary-bar__action {
    flex: 0 0 auto;
}

</style>
