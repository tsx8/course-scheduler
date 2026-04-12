<script setup>
import { computed, h, watch } from 'vue';
import { useDataStore } from '../stores/data';
import { NSelect, NDataTable, NSpace, NH2, NEmpty, NStatistic, NTag, NFlex, NLayout, NLayoutHeader, NLayoutContent, NButton, NIcon } from 'naive-ui';
import { DownloadOutline as DownloadIcon } from '@vicons/ionicons5';
import ScheduleCell from '../components/ScheduleCell.vue';

const dataStore = useDataStore();

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

const scheduledHours = computed(() => {
    if (!selectedTeacherId.value) return 0;
    const teacherSchedules = dataStore.scheduledClassesByTeacher(selectedTeacherId.value);

    const timeHoursMap = new Map(dataStore.time.map(t => [t.id, t.corresponding_hours]));
    return teacherSchedules.reduce((total, schedule) => {
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

    let csv = '\uFEFF';
    csv += `教师课表,${teacher.name}\n`;
    csv += '时间段,';
    days.value.forEach(day => {
        csv += `${day.value},`;
    });
    csv += '\n';

    timeSlots.value.forEach(time => {
        csv += `${time.value},`;
        days.value.forEach(day => {
            const schedule = teacherSchedules.find(s => s.day_id === day.id && s.time_id === time.id);
            if (schedule) {
                const course = dataStore.courses.find(c => c.id === schedule.course_id);
                const campus = dataStore.campuses.find(c => c.id === schedule.campus_id);
                const venue = dataStore.venues.find(v => v.id === schedule.venue_id);
                csv += `"${course?.name || ''} - ${campus?.name || ''} - ${venue?.name || ''}",`;
            } else {
                csv += ',';
            }
        });
        csv += '\n';
    });

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
    <n-layout style="height: calc(100vh - 96px); position: relative;">
        <n-layout-header bordered style="padding: 12px 24px;">
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
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-data-table v-if="selectedTeacherId" :columns="columns" :data="tableData" :bordered="true"
                :single-line="false" style="width: 100%;" />
            <n-flex v-else justify="center" align="center" style="flex: 1;">
                <n-empty description="请先选择一位教师以管理其课表" size="huge" />
            </n-flex>
        </n-layout-content>

        <div class="fixed-footer-stats" v-if="selectedTeacher">
            <n-space align="baseline" :size="20">
                <n-tag v-if="isOverLimit" type="error" size="small">课时超限</n-tag>
                <n-statistic label="已排课时">
                    <span
                        :style="{ color: isOverLimit ? '#d03050' : 'inherit', fontWeight: isOverLimit ? 'bold' : 'inherit' }">
                        {{ scheduledHours }}
                    </span>
                </n-statistic>
                <n-statistic label="最大课时">
                    {{ maxHours }}
                </n-statistic>
            </n-space>
        </div>
    </n-layout>
</template>

<style scoped>
.fixed-footer-stats {
    position: absolute;
    bottom: 20px;
    right: 24px;
    z-index: 100;
    background-color: #fff;
    padding: 12px 20px;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    border: 1px solid #e0e0e6;
}
</style>
