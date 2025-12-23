<template>
    <div>
        <div v-if="scheduleForCell" class="schedule-item">
            <div class="schedule-info">
                <strong>{{ courseName }}</strong>
                <span>{{ campusName }} - {{ venueName }}</span>
            </div>
            <n-button-group size="tiny" class="actions">
                <n-button circle type="info" @click="handleEdit">
                    <template #icon><n-icon :component="EditIcon" /></template>
                </n-button>
                <n-button circle type="error" @click="handleDelete">
                    <template #icon><n-icon :component="DeleteIcon" /></template>
                </n-button>
            </n-button-group>
        </div>

        <div v-else-if="isBlocked" class="blocked-schedule">
            <n-icon :component="BlockIcon" size="24" color="#a3a3a3" />
            <n-button text type="error" @click="handleBlockToggle" class="unblock-button">
                取消
            </n-button>
        </div>

        <div v-else class="add-schedule">
            <n-button-group style="gap: 4px;">
                <n-button circle dashed type="primary" @click="handleAdd" title="新增排课">
                    <template #icon><n-icon :component="AddIcon" /></template>
                </n-button>
                <n-button circle dashed type="warning" @click="handleBlockToggle" title="设为不排课">
                    <template #icon><n-icon :component="BlockIcon" /></template>
                </n-button>
            </n-button-group>
        </div>

        <n-modal v-model:show="showModal" preset="dialog" :title="modalTitle">
            <n-form ref="formRef" :model="formValue" :rules="rules" label-placement="left" label-width="auto"
                require-mark-placement="right-hanging">
                <n-form-item label="授课课程" path="course_id">
                    <n-select v-model:value="formValue.course_id" placeholder="选择课程" :options="courseOptions"
                        @update:value="handleCourseChange" />
                </n-form-item>
                <n-form-item label="上课校区" path="campus_id">
                    <n-select v-model:value="formValue.campus_id" placeholder="选择校区" :options="campusOptionsForForm"
                        :disabled="!formValue.course_id" @update:value="handleCampusChange" />
                </n-form-item>
                <n-form-item label="上课场地" path="venue_id">
                    <n-select v-model:value="formValue.venue_id" placeholder="选择场地" :options="venueOptionsForForm"
                        :disabled="!formValue.campus_id" />
                </n-form-item>
            </n-form>
            <template #action>
                <n-button @click="showModal = false">取消</n-button>
                <n-button type="primary" @click="handleSubmit">确认</n-button>
            </template>
        </n-modal>
    </div>
</template>

<script setup>
import { ref, computed } from 'vue';
import { useDataStore } from '../stores/data';
import {
    NButton, NButtonGroup, NIcon, NModal, NForm, NFormItem, NSelect, useMessage, useDialog
} from 'naive-ui';
import {
    AddOutline as AddIcon,
    CreateOutline as EditIcon,
    TrashOutline as DeleteIcon,
    BanOutline as BlockIcon,
} from '@vicons/ionicons5';

const props = defineProps({
    teacherId: { type: String, required: true },
    dayId: { type: String, required: true },
    timeId: { type: String, required: true },
});

const dataStore = useDataStore();
const message = useMessage();
const dialog = useDialog();

const showModal = ref(false);
const isEditMode = ref(false);
const formRef = ref(null);
const formValue = ref({});

const scheduleMap = computed(() => dataStore.getScheduleMapForTeacher(props.teacherId));
const scheduleForCell = computed(() => scheduleMap.value.get(`${props.dayId}-${props.timeId}`));

const unavailableSet = computed(() => dataStore.getUnavailableMapForTeacher(props.teacherId));
const isBlocked = computed(() => unavailableSet.value.has(`${props.dayId}-${props.timeId}`));

const courseName = computed(() => {
    const course = dataStore.courses.find(c => c.id === scheduleForCell.value?.course_id);
    return course?.name || '未知课程';
});
const campusName = computed(() => {
    const campus = dataStore.campuses.find(c => c.id === scheduleForCell.value?.campus_id);
    return campus?.name || '未知校区';
});
const venueName = computed(() => {
    if (!scheduleForCell.value) return '未知场地';
    const venue = dataStore.venues.find(v => v.id === scheduleForCell.value.venue_id);
    return venue?.name || '未知场地';
});

const modalTitle = computed(() => (isEditMode.value ? '编辑排课' : '新增排课'));
const courseOptions = computed(() => dataStore.teacherCourseOptions(props.teacherId));
const campusOptionsForForm = computed(() => {
    if (!formValue.value.course_id) return [];
    return dataStore.courseCampusOptions(formValue.value.course_id);
});
const venueOptionsForForm = computed(() => {
    if (!formValue.value.course_id || !formValue.value.campus_id) return [];
    return dataStore.courseVenueOptions(formValue.value.course_id, formValue.value.campus_id);
});

const rules = {
    course_id: { required: true, message: '请选择授课课程', trigger: 'change' },
    campus_id: { required: true, message: '请选择上课校区', trigger: 'change' },
    venue_id: { required: true, message: '请选择上课场地', trigger: 'change' },
};

const handleAdd = () => {
    isEditMode.value = false;
    formValue.value = {
        course_id: null,
        campus_id: null,
        venue_id: null,
    };
    showModal.value = true;
};

const handleEdit = () => {
    isEditMode.value = true;
    formValue.value = { ...scheduleForCell.value };
    showModal.value = true;
};

const handleDelete = () => {
    dialog.warning({
        title: '确认删除排课',
        content: `确定要删除这个排课记录吗？`,
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: () => {
            dataStore.deleteSchedule(scheduleForCell.value.id);
            message.success('排课已删除');
        },
    });
};

const handleBlockToggle = () => {
    dataStore.toggleUnavailableSlot(props.teacherId, props.dayId, props.timeId);
};

const handleSubmit = () => {
    if (isBlocked.value) {
        message.error('此时间段已被设为不排课，无法添加课程。');
        return;
    }

    formRef.value?.validate((errors) => {
        if (!errors) {
            const scheduleData = {
                ...formValue.value,
                day_id: props.dayId,
                time_id: props.timeId,
            };

            if (isEditMode.value) {
                dataStore.updateSchedule(props.teacherId, scheduleData);
                message.success('排课更新成功');
            } else {
                dataStore.addSchedule(props.teacherId, scheduleData);
                message.success('新增排课成功');
            }
            showModal.value = false;
        } else {
            message.error('请填写完整的排课信息');
        }
    });
};

const handleCourseChange = () => {
    formValue.value.campus_id = null;
    formValue.value.venue_id = null;
};
const handleCampusChange = () => {
    formValue.value.venue_id = null;
};

</script>

<style scoped>
.schedule-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 4px;
    border-radius: 4px;
    background-color: #e6f7ff;
    border: 1px solid #91d5ff;
    min-height: 50px;
    min-width: 100px;
    position: relative;
}

.schedule-info {
    display: flex;
    flex-direction: column;
}

.schedule-info span {
    color: #555;
}

.actions {
    position: absolute;
    top: 2px;
    right: 2px;
    opacity: 0.5;
    transition: opacity 0.2s;
}

.schedule-item:hover .actions {
    opacity: 1;
}

.add-schedule {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 60px;
}

.blocked-schedule {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60px;
    min-width: 100px;
    background-color: #fafafa;
    border: 1px dashed #d9d9d9;
    border-radius: 4px;
    position: relative;
}

.unblock-button {
    position: absolute;
    bottom: 2px;
    right: 5px;
    font-size: 12px;
    opacity: 0;
    transition: opacity 0.2s;
}

.blocked-schedule:hover .unblock-button {
    opacity: 1;
}
</style>