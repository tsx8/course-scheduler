<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">教师信息管理</n-h2>
                <n-button type="primary" @click="handleAddTeacher">
                    <template #icon><n-icon :component="AddIcon" /></template>
                    新增教师
                </n-button>
            </n-flex>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-data-table :columns="columns" :data="dataStore.teachers" :pagination="pagination" :bordered="true"
                :single-line="false" style="width: 100%;" />
            <n-modal v-model:show="showModal" preset="dialog" :title="modalTitle" style="width: 600px;">
                <n-form ref="formRef" :model="currentTeacher" :rules="rules" label-placement="left" label-width="auto"
                    require-mark-placement="right-hanging">
                    <n-form-item label="姓名" path="name">
                        <n-input v-model:value="currentTeacher.name" placeholder="输入教师姓名" />
                    </n-form-item>
                    <n-form-item label="最大授课学时" path="max_teaching_hours">
                        <n-input-number v-model:value="currentTeacher.max_teaching_hours" :min="0" placeholder="输入学时"
                            style="width: 100%;" />
                    </n-form-item>
                    <n-form-item label="可教课程" path="teaches">
                        <n-select v-model:value="currentTeacher.teaches" multiple placeholder="选择教师可教的课程"
                            :options="dataStore.courseOptions" clearable />
                    </n-form-item>
                    <n-form-item label="上课校区" path="campus_ids">
                        <n-select v-model:value="currentTeacher.campus_ids" multiple placeholder="选择教师可上课的校区"
                            :options="dataStore.campusOptions" clearable />
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

<script setup>
import { ref, computed, h } from 'vue';
import { useDataStore } from '../stores/data';
import {
    NButton, NDataTable, NSpace, NH2, NModal, NForm, NFormItem, NInput,
    NInputNumber, NIcon, NSelect, useMessage, useDialog
} from 'naive-ui';
import { AddOutline as AddIcon, CreateOutline as EditIcon, TrashOutline as DeleteIcon } from '@vicons/ionicons5';

const dataStore = useDataStore();
const message = useMessage();
const dialog = useDialog();

const showModal = ref(false);
const isEditMode = ref(false);
const currentTeacher = ref({});
const formRef = ref(null);

const modalTitle = computed(() => (isEditMode.value ? '编辑教师信息' : '新增教师'));

const defaultTeacher = () => ({
    id: '',
    name: '',
    max_teaching_hours: 0,
    teaches: [],
    campus_ids: dataStore.campuses.map(campus => campus.id),
});

const rules = {
    name: { required: true, message: '请输入教师姓名', trigger: 'blur' },
    max_teaching_hours: { type: 'number', required: true, message: '请输入最大授课学时', trigger: ['blur', 'input'] },
    campus_ids: {
        type: 'array',
        required: true,
        validator: (_rule, value) => Array.isArray(value) && value.length > 0,
        message: '请至少选择一个上课校区',
        trigger: ['change', 'blur']
    },
};

const pagination = { pageSize: 10 };

const createColumns = ({ onEdit, onDelete }) => [
    { title: '姓名', key: 'name', sorter: (a, b) => a.name.localeCompare(b.name) },
    { title: '最大学时', key: 'max_teaching_hours', sorter: (a, b) => a.max_teaching_hours - b.max_teaching_hours },
    {
        title: '上课校区',
        key: 'campus_ids',
        render: (row) => {
            const relations = dataStore.teacherCampusesByTeacher(row.id);
            if (!relations || relations.length === 0) return '暂无';
            return relations
                .map(rel => dataStore.campuses.find(campus => campus.id === rel.campus_id)?.name || '未知校区')
                .join('、');
        },
        ellipsis: { tooltip: true }
    },
    {
        title: '教学课程',
        key: 'teaches',
        width: '25%',
        render: (row) => {
            const relations = dataStore.teacherCoursesByTeacher(row.id);

            if (!relations || relations.length === 0) return '暂无';

            return relations.map(rel => {
                const course = dataStore.courses.find(c => c.id === rel.course_id);
                return course ? course.name : '未知课程';
            }).join('、 ');
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
    onEdit: (teacher) => {
        isEditMode.value = true;
        const teacherData = JSON.parse(JSON.stringify(teacher));
        teacherData.teaches = dataStore.teacherCoursesByTeacher(teacher.id).map(rel => rel.course_id);
        teacherData.campus_ids = dataStore.teacherCampusesByTeacher(teacher.id).map(rel => rel.campus_id);
        if (teacherData.campus_ids.length === 0) {
            teacherData.campus_ids = dataStore.campuses.map(campus => campus.id);
        }
        currentTeacher.value = teacherData;
        showModal.value = true;
    },
    onDelete: (teacher) => {
        dialog.warning({
            title: '确认删除',
            content: `确定要删除教师【${teacher.name}】吗？`,
            positiveText: '删除',
            negativeText: '取消',
            onPositiveClick: () => {
                dataStore.deleteTeacher(teacher.id);
                message.success(`教师 ${teacher.name} 已删除`);
            },
        });
    }
});

const handleAddTeacher = () => {
    isEditMode.value = false;
    currentTeacher.value = defaultTeacher();
    showModal.value = true;
};

const handleSubmit = () => {
    formRef.value?.validate(async (errors) => {
        if (!errors) {
            if (isEditMode.value) {
                dataStore.updateTeacher(currentTeacher.value);
                message.success('教师信息更新成功');
            } else {
                dataStore.addTeacher(currentTeacher.value);
                message.success('教师新增成功');
            }
            showModal.value = false;
        } else {
            message.error('请检查表单输入');
        }
    });
};
</script>
