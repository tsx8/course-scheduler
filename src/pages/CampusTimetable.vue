<script setup>
import { computed, h, watch } from 'vue';
import { useDataStore } from '../stores/data';
import { NSelect, NDataTable, NFlex, NH2, NEmpty, NInputNumber, NText, NButton, NIcon, useMessage } from 'naive-ui';
import { DownloadOutline as ExportIcon } from '@vicons/ionicons5';

const dataStore = useDataStore();
const message = useMessage();

const selectedCampusId = computed({
    get: () => dataStore.selectedCampusIdForCampusView,
    set: (val) => { dataStore.selectedCampusIdForCampusView = val; }
});

const selectedVenueId = computed({
    get: () => dataStore.selectedVenueIdForCampusView,
    set: (val) => { dataStore.selectedVenueIdForCampusView = val; }
});

const campusOptions = computed(() => dataStore.campusOptions);
const timeSlots = computed(() => dataStore.time);
const days = computed(() => dataStore.day);

const venueOptions = computed(() => {
    if (!selectedCampusId.value) return [];
    const venues = dataStore.venueOptionsByCampus(selectedCampusId.value);
    return [{ label: '全部场地', value: null }, ...venues];
});

watch(selectedCampusId, () => {
    selectedVenueId.value = null;
});

const scheduleMap = computed(() => dataStore.getScheduledClassesByCampus);

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
                padding: '8px'
            }
        }),
        render(row) {
            const schedulesInCell = row[day.id];

            if (selectedVenueId.value) {
                const scheduleNodes = schedulesInCell.map(({ schedule, teacher }) => {
                    const course = dataStore.courses.find(c => c.id === schedule.course_id);
                    const venue = dataStore.venues.find(v => v.id === schedule.venue_id);
                    return h('div', { style: 'text-align: left; padding: 4px; border-radius: 4px; background-color: #e6f7ff; border: 1px solid #91d5ff; margin-top: 4px;' }, [
                        h('div', { style: 'font-weight: bold;' }, `${course?.name || '未知'}`),
                        h('div', `${teacher?.name || '未知'} - ${venue?.name || '未知'}`)
                    ]);
                });
                return h('div', null, scheduleNodes);
            }

            const actualCount = schedulesInCell.length;
            const timeId = row.key;
            const dayId = day.id;

            const expectedCount = dataStore.getExpectedCountForCampusCell(selectedCampusId.value, dayId, timeId);
            const isOverbooked = actualCount > expectedCount && expectedCount > 0;

            const counterNode = h(NFlex, { align: 'center', style: 'margin-bottom: 8px;' }, {
                default: () => [
                    h(NText, { depth: 3, style: 'font-size: 12px;' }, { default: () => '期望:' }),
                    h(NInputNumber, {
                        value: expectedCount,
                        'onUpdate:value': (value) => {
                            dataStore.updateCampusScheduleDensity(selectedCampusId.value, dayId, timeId, value);
                        },
                        size: 'tiny',
                        min: 0,
                        style: 'width: 70px;',
                    }),
                    h(NText, {
                        tag: 'span',
                        style: {
                            color: isOverbooked ? '#d03050' : 'inherit',
                            fontWeight: 'normal',
                            fontSize: '12px'
                        }
                    }, { default: () => `实际: ${actualCount}` })
                ]
            });

            const scheduleNodes = schedulesInCell.map(({ schedule, teacher }) => {
                const course = dataStore.courses.find(c => c.id === schedule.course_id);
                const venue = dataStore.venues.find(v => v.id === schedule.venue_id);
                return h('div', { style: 'text-align: left; padding: 4px; border-radius: 4px; background-color: #e6f7ff; border: 1px solid #91d5ff; margin-top: 4px;' }, [
                    h('div', { style: 'font-weight: bold;' }, `${course?.name || '未知'}`),
                    h('div', `${teacher?.name || '未知'} - ${venue?.name || '未知'}`)
                ]);
            });

            return h('div', null, [
                counterNode,
                ...scheduleNodes
            ]);
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

const handleExportToCsv = () => {
    if (!selectedCampusId.value) {
        message.warning('请先选择一个校区');
        return;
    }

    const headers = ['时间', ...days.value.map(d => d.value)];
    const rows = tableData.value.map(row => {
        const rowData = [row.time_slot];
        days.value.forEach(day => {
            const schedules = row[day.id];
            if (schedules.length === 0) {
                rowData.push('');
            } else {
                const cellContent = schedules.map(({ schedule, teacher }) => {
                    const course = dataStore.courses.find(c => c.id === schedule.course_id);
                    const venue = dataStore.venues.find(v => v.id === schedule.venue_id);

                    const courseName = course?.name || '未知课程';
                    const teacherName = teacher?.name || '未知教师';
                    const venueName = venue?.name || '未知场地';

                    return `${courseName} (${teacherName} - ${venueName})`;
                }).join('\n');

                rowData.push(`"${cellContent.replace(/"/g, '""')}"`);
            }
        });
        return rowData.join(',');
    });

    const csvContent = [headers.join(','), ...rows].join('\n');

    const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' });

    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);
    link.setAttribute('href', url);

    const campusName = campusOptions.value.find(c => c.value === selectedCampusId.value)?.label || '课表';
    const venueName = venueOptions.value.find(v => v.value === selectedVenueId.value)?.label;
    const fileName = venueName ? `${campusName}_${venueName}_课表.csv` : `${campusName}_总课表.csv`;

    link.setAttribute('download', fileName);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    message.success('CSV文件已开始下载');
};


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
                <n-flex align="center">
                    <n-select v-model:value="selectedCampusId" placeholder="请选择校区" :options="campusOptions" clearable
                        style="width: 150px" />
                    <n-select v-model:value="selectedVenueId" placeholder="选择场地" :options="venueOptions" clearable
                        :disabled="!selectedCampusId" style="width: 150px" />
                    <n-button type="primary" @click="handleExportToCsv" :disabled="!selectedCampusId">
                        <template #icon><n-icon :component="ExportIcon" /></template>
                        导出CSV
                    </n-button>
                </n-flex>
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