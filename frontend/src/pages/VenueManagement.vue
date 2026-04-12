<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">场地/校区信息管理</n-h2>
                <n-button type="primary" @click="handleAddCampus">
                    <template #icon><n-icon :component="AddIcon" /></template>
                    新增校区
                </n-button>
            </n-flex>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-data-table :columns="campusColumns" :data="dataStore.campuses" :bordered="true"
                :row-key="row => row.id" @update:expanded-row-keys="handleCampusExpand"
                :expanded-row-keys="expandedCampusKeys" style="width: 100%;" />
            <n-modal v-model:show="showCampusModal" preset="dialog" :title="campusModalTitle" style="width: 500px;"
                @after-leave="onCampusModalClose">
                <n-form ref="campusFormRef" :model="currentCampus" :rules="campusRules" label-placement="left"
                    label-width="auto" require-mark-placement="right-hanging">
                    <n-form-item label="校区名称" path="name">
                        <n-input v-model:value="currentCampus.name" placeholder="输入校区名称" />
                    </n-form-item>
                </n-form>
                <template #action>
                    <n-button @click="showCampusModal = false">取消</n-button>
                    <n-button type="primary" @click="handleCampusSubmit">确认</n-button>
                </template>
            </n-modal>
            <n-modal v-model:show="showVenueModal" preset="dialog" :title="venueModalTitle" style="width: 500px;"
                @after-leave="onVenueModalClose">
                <n-form ref="venueFormRef" :model="currentVenue" :rules="venueRules" label-placement="left"
                    label-width="auto" require-mark-placement="right-hanging">
                    <n-form-item label="场地名称" path="name">
                        <n-input v-model:value="currentVenue.name" placeholder="输入场地名称" />
                    </n-form-item>
                    <n-form-item label="场地容量" path="capacity">
                        <n-input-number v-model:value="currentVenue.capacity" :min="0" placeholder="输入场地容量"
                            style="width: 100%;" />
                    </n-form-item>
                </n-form>
                <template #action>
                    <n-button @click="showVenueModal = false">取消</n-button>
                    <n-button type="primary" @click="handleVenueSubmit">确认</n-button>
                </template>
            </n-modal>
        </n-layout-content>
    </n-layout>
</template>

<script setup>
import { ref, computed, h } from 'vue';
import { useDataStore } from '../stores/data';
import {
    NButton, NDataTable, NSpace, NH2, NModal, NForm, NFormItem, NInput,
    NInputNumber, NIcon, useMessage, useDialog
} from 'naive-ui';
import { AddOutline as AddIcon, CreateOutline as EditIcon, TrashOutline as DeleteIcon, BusinessOutline as VenueIcon } from '@vicons/ionicons5';

const dataStore = useDataStore();
const message = useMessage();
const dialog = useDialog();

const showCampusModal = ref(false);
const isEditModeCampus = ref(false);
const currentCampus = ref({});
const campusFormRef = ref(null);
const expandedCampusKeys = ref([]);

const campusModalTitle = computed(() => (isEditModeCampus.value ? '编辑校区信息' : '新增校区'));
const defaultCampus = () => ({ id: '', name: '', venues: [] });

const campusRules = {
    name: { required: true, message: '请输入校区名称', trigger: 'blur' },
};

const showVenueModal = ref(false);
const isEditModeVenue = ref(false);
const currentVenue = ref({});
const venueFormRef = ref(null);
const activeCampusIdForVenue = ref(null);

const venueModalTitle = computed(() => (isEditModeVenue.value ? '编辑场地信息' : '新增场地'));
const defaultVenue = () => ({ id: '', name: '', capacity: 0 });

const venueRules = {
    name: { required: true, message: '请输入场地名称', trigger: 'blur' },
    capacity: { type: 'number', required: true, message: '请输入场地容量', trigger: ['blur', 'input'], min: 0 },
};

const campusColumns = [
    {
        type: 'expand',
        renderExpand: (campusRowData) => {
            return h(NDataTable, {
                columns: createVenueColumns(campusRowData.id),
                data: dataStore.venuesByCampus(campusRowData.id),
                singleLine: false,
                size: 'small',
                pagination: { pageSize: 5 },
            });
        }
    },
    { title: '校区名称', key: 'name', sorter: (a, b) => a.name.localeCompare(b.name) },
    {
        title: '操作',
        key: 'actions',
        width: '25%',
        render(row) {
            return h(NSpace, null, {
                default: () => [
                    h(NButton, {
                        size: 'small',
                        type: 'primary',
                        ghost: true,
                        onClick: () => handleAddVenue(row.id)
                    }, { icon: () => h(NIcon, { component: VenueIcon }), default: () => '新增场地' }),
                    h(NButton, {
                        size: 'small',
                        type: 'info',
                        onClick: () => handleEditCampus(row)
                    }, { icon: () => h(NIcon, { component: EditIcon }) }),
                    h(NButton, {
                        size: 'small',
                        type: 'error',
                        onClick: () => handleDeleteCampus(row)
                    }, { icon: () => h(NIcon, { component: DeleteIcon }) })
                ]
            });
        }
    }
];

const handleCampusExpand = (keys) => {
    expandedCampusKeys.value = keys;
};

const createVenueColumns = (campusId) => [
    // { title: '场地ID', key: 'id', ellipsis: { tooltip: true }, width: 200 },
    { title: '场地名称', key: 'name', sorter: (a, b) => a.name.localeCompare(b.name) },
    { title: '容量', key: 'capacity', sorter: (a, b) => a.capacity - b.capacity, width: '10%' },
    {
        title: '操作',
        key: 'actions',
        width: '15%',
        render(row) {
            return h(NSpace, null, {
                default: () => [
                    h(NButton, {
                        size: 'small',
                        type: 'info',
                        onClick: () => handleEditVenue(campusId, row)
                    }, { icon: () => h(NIcon, { component: EditIcon }) }),
                    h(NButton, {
                        size: 'small',
                        type: 'error',
                        onClick: () => handleDeleteVenue(campusId, row)
                    }, { icon: () => h(NIcon, { component: DeleteIcon }) })
                ]
            });
        }
    }
];

const handleAddCampus = () => {
    isEditModeCampus.value = false;
    currentCampus.value = defaultCampus();
    showCampusModal.value = true;
};

const handleEditCampus = (campus) => {
    isEditModeCampus.value = true;
    currentCampus.value = JSON.parse(JSON.stringify(campus));
    showCampusModal.value = true;
};

const handleDeleteCampus = (campus) => {
    dialog.warning({
        title: '确认删除校区',
        content: () => h('div', [
            `确定要删除校区【${campus.name}】吗？此操作将：`, h('br'),
            '1. 删除该校区下的所有场地。', h('br'),
            '2. 从所有课程的地点中移除与此校区及其场地相关的记录。', h('br'),
            '操作不可撤销。'
        ]),
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: () => {
            dataStore.deleteCampus(campus.id);
            message.success(`校区 ${campus.name} 已删除`);
            expandedCampusKeys.value = expandedCampusKeys.value.filter(key => key !== campus.id);
        },
    });
};

const handleCampusSubmit = () => {
    campusFormRef.value?.validate(async (errors) => {
        if (!errors) {
            if (isEditModeCampus.value) {
                dataStore.updateCampus(currentCampus.value);
                message.success('校区信息更新成功');
            } else {
                dataStore.addCampus(currentCampus.value);
                message.success('校区新增成功');
            }
            showCampusModal.value = false;
        } else {
            message.error('请检查表单输入');
        }
    });
};

const onCampusModalClose = () => {
    currentCampus.value = defaultCampus();
};

const handleAddVenue = (campusId) => {
    isEditModeVenue.value = false;
    currentVenue.value = defaultVenue();
    activeCampusIdForVenue.value = campusId;
    showVenueModal.value = true;
};

const handleEditVenue = (campusId, venue) => {
    isEditModeVenue.value = true;
    currentVenue.value = JSON.parse(JSON.stringify(venue));
    activeCampusIdForVenue.value = campusId;
    showVenueModal.value = true;
};

const handleDeleteVenue = (campusId, venue) => {
    const campus = dataStore.campuses.find(c => c.id === campusId);
    dialog.warning({
        title: '确认删除场地',
        content: () => h('div', [
            `确定要删除校区【${campus?.name || '未知校区'}】下的场地【${venue.name}】吗？此操作将从所有课程的地点中移除与此场地相关的记录。`, h('br'),
            '操作不可撤销。'
        ]),
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: () => {
            dataStore.deleteVenueFromCampus(campusId, venue.id);
            message.success(`场地 ${venue.name} 已删除`);
        },
    });
};

const handleVenueSubmit = () => {
    venueFormRef.value?.validate(async (errors) => {
        if (!errors) {
            if (!activeCampusIdForVenue.value) {
                message.error('未指定校区，无法保存场地');
                return;
            }
            if (isEditModeVenue.value) {
                dataStore.updateVenueInCampus(activeCampusIdForVenue.value, currentVenue.value);
                message.success('场地信息更新成功');
            } else {
                dataStore.addVenueToCampus(activeCampusIdForVenue.value, currentVenue.value);
                message.success('场地新增成功');
            }
            showVenueModal.value = false;
        } else {
            message.error('请检查表单输入');
        }
    });
};

const onVenueModalClose = () => {
    currentVenue.value = defaultVenue();
    activeCampusIdForVenue.value = null;
};

</script>