<script setup>
import { ref, computed, h } from 'vue';
import {
    NLayout, NLayoutHeader, NLayoutContent, NFlex, NH2, NButton, NCard, NText, useMessage, useDialog,
    NDataTable, NModal, NForm, NFormItem, NInput, NInputNumber, NIcon, NSpace, NDivider, NTooltip
} from 'naive-ui';
import { AddOutline as AddIcon, CreateOutline as EditIcon, TrashOutline as DeleteIcon } from '@vicons/ionicons5';
import { useDataStore } from '../stores/data';
import { save, open, invoke } from '../host/desktop';

const message = useMessage();
const dialog = useDialog();
const dataStore = useDataStore();

const handleExportData = async () => {
    try {
        const filePath = await save({
            title: '导出数据到...',
            filters: [
                { name: 'SQLite Database', extensions: ['db'] },
                { name: 'JSON', extensions: ['json'] }
            ],
            defaultPath: 'course_scheduler_export.db'
        });

        if (filePath) {
            const isJson = filePath.toLowerCase().endsWith('.json');

            if (isJson) {
                await invoke('export_json', { filePath });
                message.success('数据已导出为JSON格式！');
            } else {
                await invoke('export_database', { filePath });
                message.success('数据已导出为数据库格式！');
            }
        }
    } catch (error) {
        console.error('导出失败:', error);
        message.error(`导出失败: ${error}`);
    }
};

const handleImportData = async () => {
    try {
        const selectedPath = await open({
            title: '从文件导入数据',
            multiple: false,
            filters: [
                { name: 'All Supported', extensions: ['db', 'json'] },
                { name: 'SQLite Database', extensions: ['db'] },
                { name: 'JSON', extensions: ['json'] }
            ]
        });

        if (selectedPath) {
            const isJson = selectedPath.toLowerCase().endsWith('.json');

            let stats;
            if (isJson) {
                stats = await invoke('import_json', { filePath: selectedPath });
                message.success(`JSON数据已导入到临时区域！${stats.teachers}位教师，${stats.courses}门课程，${stats.schedules}个排课。请检查后点击"保存"提交。`);
            } else {
                stats = await invoke('import_database', { filePath: selectedPath });
                message.success(`数据库已导入到临时区域！${stats.teachers}位教师，${stats.courses}门课程，${stats.schedules}个排课。请检查后点击"保存"提交。`);
            }

            const reloadedData = await invoke('load_data');
            dataStore.replaceAllData(reloadedData);
        }
    } catch (error) {
        console.error('导入失败:', error);
        dialog.error({
            title: '导入失败',
            content: `导入过程中发生错误:\n${error}`,
            positiveText: '确定'
        });
    }
};

const showTimeModal = ref(false);
const isEditModeTime = ref(false);
const currentTimeSlot = ref({});
const timeFormRef = ref(null);

const timeModalTitle = computed(() => (isEditModeTime.value ? '编辑上课时间' : '新增上课时间'));
const defaultTimeSlot = () => ({ value: '', corresponding_hours: 2 });
const timeRules = {
    value: { required: true, message: '请输入时间名称', trigger: 'blur' },
    corresponding_hours: { type: 'number', required: true, message: '请输入对应学时', trigger: ['blur', 'input'] }
};

const timeColumns = [
    { title: '名称', key: 'value' },
    { title: '对应学时', key: 'corresponding_hours', width: 120, align: 'center' },
    {
        title: '操作', key: 'actions', width: 150, align: 'center',
        render(row) {
            return h(NSpace, { justify: 'center' }, {
                default: () => [
                    h(NButton, { size: 'small', type: 'info', onClick: () => handleEditTimeSlot(row) }, { icon: () => h(NIcon, { component: EditIcon }) }),
                    h(NButton, { size: 'small', type: 'error', onClick: () => handleDeleteTimeSlot(row) }, { icon: () => h(NIcon, { component: DeleteIcon }) })
                ]
            });
        }
    }
];

const handleAddTimeSlot = () => {
    isEditModeTime.value = false;
    currentTimeSlot.value = defaultTimeSlot();
    showTimeModal.value = true;
};

const handleEditTimeSlot = (slot) => {
    isEditModeTime.value = true;
    currentTimeSlot.value = JSON.parse(JSON.stringify(slot));
    showTimeModal.value = true;
};

const handleDeleteTimeSlot = (slot) => {
    dialog.warning({
        title: '确认删除',
        content: `确定要删除时间段【${slot.value}】吗？所有引用该时间段的排课记录都将被删除。`,
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: () => {
            dataStore.deleteTimeSlot(slot.id);
            message.success(`时间段 ${slot.value} 已删除`);
        },
    });
};

const handleTimeSubmit = () => {
    timeFormRef.value?.validate((errors) => {
        if (!errors) {
            if (isEditModeTime.value) {
                dataStore.updateTimeSlot(currentTimeSlot.value);
                message.success('更新成功');
            } else {
                dataStore.addTimeSlot(currentTimeSlot.value);
                message.success('新增成功');
            }
            showTimeModal.value = false;
        } else {
            message.error('请检查输入');
        }
    });
};

const showDayModal = ref(false);
const isEditModeDay = ref(false);
const currentDay = ref({});
const dayFormRef = ref(null);

const dayModalTitle = computed(() => (isEditModeDay.value ? '编辑工作日' : '新增工作日'));
const defaultDay = () => ({ value: '' });
const dayRules = {
    value: { required: true, message: '请输入工作日名称', trigger: 'blur' }
};

const dayColumns = [
    { title: '名称', key: 'value' },
    {
        title: '操作', key: 'actions', width: 150, align: 'center',
        render(row) {
            return h(NSpace, { justify: 'center' }, {
                default: () => [
                    h(NButton, { size: 'small', type: 'info', onClick: () => handleEditDay(row) }, { icon: () => h(NIcon, { component: EditIcon }) }),
                    h(NButton, { size: 'small', type: 'error', onClick: () => handleDeleteDay(row) }, { icon: () => h(NIcon, { component: DeleteIcon }) })
                ]
            });
        }
    }
];

const handleAddDay = () => {
    isEditModeDay.value = false;
    currentDay.value = defaultDay();
    showDayModal.value = true;
};

const handleEditDay = (day) => {
    isEditModeDay.value = true;
    currentDay.value = JSON.parse(JSON.stringify(day));
    showDayModal.value = true;
};

const handleDeleteDay = (day) => {
    dialog.warning({
        title: '确认删除',
        content: `确定要删除工作日【${day.value}】吗？所有引用该工作日的排课记录都将被删除。`,
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: () => {
            dataStore.deleteDay(day.id);
            message.success(`工作日 ${day.value} 已删除`);
        },
    });
};

const handleDaySubmit = () => {
    dayFormRef.value?.validate((errors) => {
        if (!errors) {
            if (isEditModeDay.value) {
                dataStore.updateDay(currentDay.value);
                message.success('更新成功');
            } else {
                dataStore.addDay(currentDay.value);
                message.success('新增成功');
            }
            showDayModal.value = false;
        } else {
            message.error('请检查输入');
        }
    });
};
</script>

<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">应用设置</n-h2>
            </n-flex>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-flex vertical :size="24">
                <n-card title="上课时间管理">
                    <n-flex vertical>
                        <n-text>管理课表中的时间段，例如“第一大节”、“第三大节”等。删除时间段会一并删除所有相关的排课记录。</n-text>
                        <n-button type="primary" @click="handleAddTimeSlot" style="max-width: 120px;">
                            <template #icon><n-icon :component="AddIcon" /></template>
                            新增时间段
                        </n-button>
                        <n-data-table :columns="timeColumns" :data="dataStore.time" :bordered="true"
                            :single-line="false" />
                    </n-flex>
                </n-card>

                <n-card title="工作日管理">
                    <n-flex vertical>
                        <n-text>管理课表中的工作日，例如“星期一”、“星期二”等。删除工作日会一并删除所有相关的排课记录。</n-text>
                        <n-button type="primary" @click="handleAddDay" style="max-width: 120px;">
                            <template #icon><n-icon :component="AddIcon" /></template>
                            新增工作日
                        </n-button>
                        <n-data-table :columns="dayColumns" :data="dataStore.day" :bordered="true"
                            :single-line="false" />
                    </n-flex>
                </n-card>

                <n-card title="数据管理">
                    <n-flex vertical :size="16">
                        <div>
                            <n-text strong>导入/导出</n-text>
                            <n-text style="display: block; margin-top: 8px;">
                                将数据导出为 SQLite 数据库（.db）或 JSON 文件（.json）进行备份，或从备份文件恢复数据。
                                导入操作会将数据加载到临时区域，请检查后点击"保存"按钮提交。
                            </n-text>
                            <n-flex style="margin-top: 12px;">
                                <n-tooltip :disabled="!dataStore.hasUnsavedChanges">
                                    <template #trigger>
                                        <n-button type="primary" @click="handleExportData"
                                            :disabled="dataStore.hasUnsavedChanges">
                                            导出数据...
                                        </n-button>
                                    </template>
                                    请先保存当前修改后再导出数据
                                </n-tooltip>
                                <n-button @click="handleImportData">导入数据...</n-button>
                            </n-flex>
                        </div>

                        <n-divider style="margin: 8px 0;" />

                        <n-text depth="3">运行日志入口已移除。</n-text>
                    </n-flex>
                </n-card>
            </n-flex>

            <n-modal v-model:show="showTimeModal" preset="dialog" :title="timeModalTitle" style="width: 500px;">
                <n-form ref="timeFormRef" :model="currentTimeSlot" :rules="timeRules" label-placement="left"
                    label-width="auto" require-mark-placement="right-hanging">
                    <n-form-item label="时间名称" path="value">
                        <n-input v-model:value="currentTimeSlot.value" placeholder="例如: 第一大节" />
                    </n-form-item>
                    <n-form-item label="对应学时" path="corresponding_hours">
                        <n-input-number v-model:value="currentTimeSlot.corresponding_hours" :min="1" placeholder="输入学时"
                            style="width: 100%;" />
                    </n-form-item>
                </n-form>
                <template #action>
                    <n-button @click="showTimeModal = false">取消</n-button>
                    <n-button type="primary" @click="handleTimeSubmit">确认</n-button>
                </template>
            </n-modal>

            <n-modal v-model:show="showDayModal" preset="dialog" :title="dayModalTitle" style="width: 500px;">
                <n-form ref="dayFormRef" :model="currentDay" :rules="dayRules" label-placement="left" label-width="auto"
                    require-mark-placement="right-hanging">
                    <n-form-item label="工作日名称" path="value">
                        <n-input v-model:value="currentDay.value" placeholder="例如: 星期一" />
                    </n-form-item>
                </n-form>
                <template #action>
                    <n-button @click="showDayModal = false">取消</n-button>
                    <n-button type="primary" @click="handleDaySubmit">确认</n-button>
                </template>
            </n-modal>
        </n-layout-content>
    </n-layout>
</template>
