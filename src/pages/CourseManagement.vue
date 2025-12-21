<script setup>
import { ref, computed, h, watch } from 'vue';
import { useDataStore } from '../stores/data';
import {
    NButton, NDataTable, NSpace, NH2, NModal, NForm, NFormItem, NInput,
    NIcon, NSelect, NDynamicInput, useMessage, useDialog, NFlex
} from 'naive-ui';
import { AddOutline as AddIcon, CreateOutline as EditIcon, TrashOutline as DeleteIcon } from '@vicons/ionicons5';

const dataStore = useDataStore();
const message = useMessage();
const dialog = useDialog();

const showModal = ref(false);
const isEditMode = ref(false);
const currentCourse = ref({});
const formRef = ref(null);

const venueOptionsForPlace = ref([]);

const modalTitle = computed(() => (isEditMode.value ? '编辑课程信息' : '新增课程'));

const defaultCourse = () => ({
    name: '',
    place: [],
});

const rules = {
    name: { required: true, message: '请输入课程名称', trigger: 'blur' },
    place: {
        type: 'array',
        trigger: 'change'
    }
};

const pagination = { pageSize: 10 };

const handleCampusChange = (index, campusId) => {
    if (currentCourse.value.place[index]) {
        currentCourse.value.place[index].venue_id = null;
        if (campusId) {
            venueOptionsForPlace.value[index] = dataStore.venueOptionsByCampus(campusId);
        } else {
            venueOptionsForPlace.value[index] = [];
        }
    }
};

watch(() => currentCourse.value.place, (newPlaces) => {
    if (newPlaces && Array.isArray(newPlaces)) {
        venueOptionsForPlace.value = newPlaces.map(p =>
            p.campus_id ? dataStore.venueOptionsByCampus(p.campus_id) : []
        );
    }
}, { deep: true, immediate: true });


const createColumns = ({ onEdit, onDelete }) => [
    // { title: 'ID', key: 'id', ellipsis: { tooltip: true }, width: 250 },
    { title: '课程名称', key: 'name', sorter: (a, b) => a.name.localeCompare(b.name) },
    {
        title: '上课地点',
        key: 'place',
        render: (row) => {
            const relations = dataStore.courseVenuesByCourse(row.id);

            if (!relations || relations.length === 0) return '未指定';

            return relations.map(rel => {
                const venue = dataStore.venues.find(v => v.id === rel.venue_id);
                const campus = dataStore.campuses.find(c => c.id === venue?.campus_id);
                return `${campus?.name || '未知校区'} - ${venue?.name || '未知场地'}`;
            }).join('; ');
        },
        ellipsis: { tooltip: true }
    },
    {
        title: '操作',
        key: 'actions',
        width: '15%',
        render(row) {
            return h(NSpace, null, {
                default: () => [
                    h(NButton, { size: 'small', type: 'info', onClick: () => onEdit(row) }, { icon: () => h(NIcon, { component: EditIcon }) }),
                    h(NButton, { size: 'small', type: 'error', onClick: () => onDelete(row) }, { icon: () => h(NIcon, { component: DeleteIcon }) })
                ]
            });
        }
    }
];

const columns = createColumns({
    onEdit: (course) => {
        isEditMode.value = true;
        const courseData = JSON.parse(JSON.stringify(course));
        courseData.place = dataStore.courseVenuesByCourse(course.id).map(rel => {
            const venue = dataStore.venues.find(v => v.id === rel.venue_id);
            return {
                campus_id: venue?.campus_id || null,
                venue_id: rel.venue_id
            };
        });

        currentCourse.value = courseData;
        if (currentCourse.value.place) {
            venueOptionsForPlace.value = currentCourse.value.place.map(pItem =>
                pItem.campus_id ? dataStore.venueOptionsByCampus(pItem.campus_id) : []
            );
        } else {
            venueOptionsForPlace.value = [];
        }
        showModal.value = true;
    },
    onDelete: (course) => {
        dialog.warning({
            title: '确认删除',
            content: `确定要删除课程【${course.name}】吗？这将同时从教师的“可教课程”中移除该课程。此操作不可撤销。`,
            positiveText: '删除',
            negativeText: '取消',
            onPositiveClick: () => {
                dataStore.deleteCourse(course.id);
                message.success(`课程 ${course.name} 已删除`);
            },
        });
    }
});

const handleAddCourse = () => {
    isEditMode.value = false;
    currentCourse.value = defaultCourse();
    venueOptionsForPlace.value = [];
    showModal.value = true;
};

const onCreatePlace = () => {
    venueOptionsForPlace.value.push([]);
    return { campus_id: null, venue_id: null };
};

const handleSubmit = () => {
    formRef.value?.validate(async (errors) => {
        if (!errors) {
            let isPlaceLogicValid = true;
            let placeErrorMessage = '';

            for (const p of currentCourse.value.place) {
                const campusSelected = p.campus_id && String(p.campus_id).trim() !== '';
                const venueSelected = p.venue_id && String(p.venue_id).trim() !== '';

                if (campusSelected && !venueSelected) {
                    isPlaceLogicValid = false;
                    const campusName = dataStore.campuses.find(c => c.id === p.campus_id)?.name || p.campus_id;
                    placeErrorMessage = `上课地点错误：校区 "${campusName}" 已选择，但场地未选择。请补充完整或移除该地点条目。`;
                    break;
                }
                if (!campusSelected && venueSelected) {
                    isPlaceLogicValid = false;
                    const venueName = venueOptionsForPlace.value
                        .flat()
                        .find(opt => opt.value === p.venue_id)?.label || p.venue_id;
                    placeErrorMessage = `上课地点错误：场地 "${venueName}" 已选择，但校区未选择。请补充完整或移除该地点条目。`;
                    break;
                }
            }

            if (!isPlaceLogicValid) {
                message.error(placeErrorMessage);
                return;
            }

            currentCourse.value.place = currentCourse.value.place.filter(p => {
                const campusSelected = p.campus_id && String(p.campus_id).trim() !== '';
                const venueSelected = p.venue_id && String(p.venue_id).trim() !== '';
                return campusSelected && venueSelected;
            });

            const courseDataToSave = JSON.parse(JSON.stringify(currentCourse.value));

            if (isEditMode.value) {
                dataStore.updateCourse(courseDataToSave);
                message.success('课程信息更新成功');
            } else {
                dataStore.addCourse(courseDataToSave);
                message.success('课程新增成功');
            }
            showModal.value = false;
        } else {
            message.error('表单校验失败，请检查课程名称等必填项。');
        }
    });
};
</script>

<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">课程信息管理</n-h2>
                <n-button type="primary" @click="handleAddCourse">
                    <template #icon><n-icon :component="AddIcon" /></template>
                    新增课程
                </n-button>
            </n-flex>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-data-table :columns="columns" :single-line="false" :data="dataStore.courses" :pagination="pagination"
                :bordered="true" style="width: 100%;" />
            <n-modal v-model:show="showModal" preset="dialog" :title="modalTitle" style="width: 700px;">
                <n-form ref="formRef" :model="currentCourse" :rules="rules" label-placement="left" label-width="auto"
                    require-mark-placement="right-hanging">
                    <n-form-item label="课程名称" path="name">
                        <n-input v-model:value="currentCourse.name" placeholder="输入课程名称" />
                    </n-form-item>
                    <n-form-item label="上课地点" path="place">
                        <n-dynamic-input v-model:value="currentCourse.place" :on-create="onCreatePlace" #="{ index }">
                            <n-space style="width:100%" align="center">
                                <n-select v-model:value="currentCourse.place[index].campus_id" placeholder="选择校区"
                                    :options="dataStore.campusOptions" clearable style="min-width: 200px;"
                                    @update:value="(campusId) => handleCampusChange(index, campusId)" />
                                <n-select v-model:value="currentCourse.place[index].venue_id" placeholder="选择场地"
                                    :options="venueOptionsForPlace[index] || []"
                                    :disabled="!currentCourse.place[index] || !currentCourse.place[index].campus_id"
                                    clearable style="min-width: 200px;" />
                            </n-space>
                        </n-dynamic-input>
                    </n-form-item>
                </n-form>
                <template #action>
                    <n-button @click="showModal = false">取消</n-button>
                    <n-button type="primary" @click="handleSubmit">确认</n-button>
                </template>
            </n-modal>
        </n-layout-content>
    </n-layout>
</template>
