<script setup>
import { computed, h, watch } from 'vue';
import { useDataStore } from '../stores/data';
import { NSelect, NDataTable, NSpace, NH2, NEmpty, NStatistic, NTag, NFlex, NLayout, NLayoutHeader, NLayoutContent } from 'naive-ui';
import ScheduleCell from '../components/ScheduleCell.vue';

const dataStore = useDataStore();

const teacherOptions = computed(() => dataStore.teacherOptions);
const timeSlots = computed(() => dataStore.time);
const days = computed(() => dataStore.day);

const selectedTeacherId = computed({
    get: () => dataStore.selectedTeacherIdForTeacherView,
    set: (val) => { dataStore.selectedTeacherIdForTeacherView = val; }
});

const selectedTeacher = computed(() => {
    if (!selectedTeacherId.value) return null;
    return dataStore.teachers.find(t => t.id === selectedTeacherId.value);
});

const scheduledHours = computed(() => {
    if (!selectedTeacher.value || !selectedTeacher.value.scheduled?.length) {
        return 0;
    }
    const timeHoursMap = new Map(dataStore.time.map(t => [t.id, t.corresponding_hours]));

    return selectedTeacher.value.scheduled.reduce((total, schedule) => {
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

watch(() => dataStore.teachers, (newTeachers) => {
    if (selectedTeacherId.value && !newTeachers.find(t => t.id === selectedTeacherId.value)) {
        selectedTeacherId.value = null;
    }
}, { deep: true });

</script>

<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">教师个人课表</n-h2>
                <n-select v-model:value="selectedTeacherId" placeholder="请选择教师" :options="teacherOptions" filterable
                    clearable style="width: 250px" />
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
    </n-layout>
    <div class="fixed-footer-stats">
        <n-space v-if="selectedTeacher" align="baseline" :size="20">
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
</template>

<style scoped>
.fixed-footer-stats {
    position: fixed;
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
