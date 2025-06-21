<script setup>
import { computed, h, watch } from 'vue';
import { useDataStore } from '../stores/data';
import { NSelect, NDataTable, NSpace, NH2, NTag, NEmpty } from 'naive-ui';

const dataStore = useDataStore();
const selectedCampusId = computed({
    get: () => dataStore.selectedCampusIdForCampusView,
    set: (val) => { dataStore.selectedCampusIdForCampusView = val; }
});

const campusOptions = computed(() => dataStore.campusOptions);
const timeSlots = computed(() => dataStore.time);
const days = computed(() => dataStore.day);

const scheduleMap = computed(() => {
    if (!selectedCampusId.value) return new Map();
    return dataStore.getScheduledClassesByCampus(selectedCampusId.value);
});

const tableData = computed(() => {
    if (!selectedCampusId.value) return [];
    return timeSlots.value.map(time => {
        const row = {
            key: time.id,
            time_slot: time.value
        };
        days.value.forEach(day => {
            const mapKey = `${day.id}-${time.id}`;
            row[day.id] = scheduleMap.value.get(mapKey) || [];
        });
        return row;
    });
});

const columns = computed(() => {
    if (!selectedCampusId.value) return [];
    const dayColumns = days.value.map(day => ({
        title: day.value,
        key: day.id,
        align: 'center',
        cellProps: () => ({
            style: {
                verticalAlign: 'top',
                paddingTop: '12px'
            }
        }),
        render(row) {
            const schedulesInCell = row[day.id];
            return h(NSpace, { vertical: true, size: 'small' }, {
                default: () => schedulesInCell.map(({ schedule, teacher }) => {
                    const course = dataStore.courses.find(c => c.id === schedule.course_id);
                    const venue = dataStore.campuses.find(c => c.id === schedule.campus_id)
                        ?.venues.find(v => v.id === schedule.venue_id);
                    return h('div', { style: 'text-align: left; padding: 4px; border-radius: 4px; background-color: #e6f7ff; border: 1px solid #91d5ff;' }, [
                        h('div', { style: 'font-weight: bold;' }, `${course?.name || '未知'}`),
                        h('div', `${teacher?.name || '未知'} - ${venue?.name || '未知'}`)
                    ]);
                })
            });
        }
    }));

    return [
        {
            title: '时间',
            key: 'time_slot',
            align: 'center',
            fixed: 'left',
            resizable: true,
            width: '10%',
            render(row) {
                return h('div', { style: 'font-weight: bold;' }, row.time_slot);
            }
        },
        ...dayColumns
    ];
});

watch(() => dataStore.campuses, (newCampuses) => {
    if (selectedCampusId.value && !newCampuses.find(c => c.id === selectedCampusId.value)) {
        selectedCampusId.value = null;
    }
}, { deep: true });

</script>

<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">校区总课表</n-h2>
                <n-select v-model:value="selectedCampusId" placeholder="请选择校区" :options="campusOptions" clearable
                    style="width: 250px" />
            </n-flex>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-data-table v-if="selectedCampusId" :columns="columns" :data="tableData" :bordered="true"
                :single-line="false" style="width: 100%;" />
            <n-flex v-else justify="center" align="center" style="height: 100%; flex: 1;">
                <n-empty description="请先选择一个校区以查看课表" size="huge" />
            </n-flex>
        </n-layout-content>
    </n-layout>
</template>
