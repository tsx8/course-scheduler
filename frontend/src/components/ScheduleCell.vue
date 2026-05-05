<template>
    <ScheduleDropCell
        :target="cellTarget"
        :issues="cellIssues"
        :focused="isCellFocused"
        :can-drop="!isBlocked"
        :occupied="hasSchedules"
        :show-issue-badge="false"
        @blank-area-drop="handleBlankAreaDrop"
        @card-drop="handleCardDrop"
    >
        <template v-if="hasSchedules">
            <ScheduleCard
                v-for="schedule in schedulesForCell"
                :key="schedule.id"
                :schedule="schedule"
                context="teacher"
                :issues="issuesForSchedule(schedule.id)"
                :focused="isScheduleFocused(schedule.id)"
                @pointer-drag-start="handlePointerDragStart"
                @lock-toggle="handleLockToggle"
                @stage="handleStage"
                @edit="handleEdit"
                @delete="handleDelete"
            />
        </template>

        <template #blank>
            <div v-if="isBlocked" class="blocked-schedule">
                <n-icon :component="BlockIcon" size="24" color="#a3a3a3" />
                <n-button text type="error" @click.stop="handleBlockToggle" class="unblock-button">
                    取消
                </n-button>
            </div>

            <div v-else class="add-schedule">
                <n-button-group style="gap: 4px;">
                    <n-button circle dashed type="primary" @click.stop="handleAdd" title="新增排课">
                        <template #icon><n-icon :component="AddIcon" /></template>
                    </n-button>
                    <n-button circle dashed type="warning" @click.stop="handleBlockToggle" title="设为不排课">
                        <template #icon><n-icon :component="BlockIcon" /></template>
                    </n-button>
                </n-button-group>
            </div>
        </template>

        <template #issue-badge="{ count }">
            <span class="cell-issue-badge">{{ count }}</span>
        </template>
    </ScheduleDropCell>

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

    <n-modal v-model:show="showConflictModal" preset="dialog" title="处理目标排课">
        <div class="conflict-choice">
            目标时间已有排课，请选择处理方式。
        </div>
        <template #action>
            <n-button type="warning" @click="handleConflictReplace">覆盖</n-button>
            <n-button type="primary" @click="handleConflictSwap">交换</n-button>
            <n-button type="info" @click="handleConflictDisplace">换下</n-button>
            <n-button @click="showConflictModal = false">取消</n-button>
        </template>
    </n-modal>
</template>

<script setup>
import { ref, computed } from 'vue';
import { useDataStore } from '../stores/data';
import { useScheduleDragStore } from '../stores/scheduleDrag';
import {
    NButton, NButtonGroup, NIcon, NModal, NForm, NFormItem, NSelect, useMessage, useDialog
} from 'naive-ui';
import {
    AddOutline as AddIcon,
    BanOutline as BlockIcon,
} from '@vicons/ionicons5';
import ScheduleCard from './ScheduleCard.vue';
import ScheduleDropCell from './ScheduleDropCell.vue';

const props = defineProps({
    teacherId: { type: String, required: true },
    dayId: { type: String, required: true },
    timeId: { type: String, required: true },
});

const dataStore = useDataStore();
const scheduleDrag = useScheduleDragStore();
const message = useMessage();
const dialog = useDialog();

const showModal = ref(false);
const showConflictModal = ref(false);
const isEditMode = ref(false);
const formRef = ref(null);
const formValue = ref({});
const conflictChoice = ref({ sourceScheduleId: null, targetScheduleId: null });

const cellKey = computed(() => `${props.dayId}-${props.timeId}`);
const teacherCellKey = computed(() => `${props.teacherId}-${props.dayId}-${props.timeId}`);
const cellTarget = computed(() => ({
    type: 'teacher',
    teacher_id: props.teacherId,
    day_id: props.dayId,
    time_id: props.timeId,
}));

const scheduleListMap = computed(() => dataStore.getScheduleListMapForTeacher(props.teacherId));
const schedulesForCell = computed(() => scheduleListMap.value.get(cellKey.value) || []);
const hasSchedules = computed(() => schedulesForCell.value.length > 0);

const unavailableSet = computed(() => dataStore.getUnavailableMapForTeacher(props.teacherId));
const isBlocked = computed(() => unavailableSet.value.has(cellKey.value) && !hasSchedules.value);

const cellIssues = computed(() => dataStore.issuesByTeacherCell.get(teacherCellKey.value) || []);
const focusedTarget = computed(() => dataStore.focusedScheduleTarget || null);
const focusedScheduleId = computed(() => focusedTarget.value?.schedule_id || focusedTarget.value?.scheduleId || null);
const isCellFocused = computed(() => {
    const target = focusedTarget.value;
    if (!target) return false;
    return (!target.type || target.type === 'teacher')
        && target.teacher_id === props.teacherId
        && target.day_id === props.dayId
        && target.time_id === props.timeId;
});

const modalTitle = computed(() => (isEditMode.value ? '编辑排课' : '新增排课'));
const courseOptions = computed(() => dataStore.teacherCourseOptions(props.teacherId));
const campusOptionsForForm = computed(() => {
    if (!formValue.value.course_id) return [];
    return dataStore.courseCampusOptions(formValue.value.course_id, props.teacherId);
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

const readMaybeRef = (value) => value?.value ?? value;

const getDraggedScheduleId = () => {
    const state = readMaybeRef(scheduleDrag.dragState) || readMaybeRef(scheduleDrag.currentDrag) || {};
    return readMaybeRef(scheduleDrag.draggedScheduleId)
        || state.scheduleId
        || state.schedule_id
        || state.sourceScheduleId
        || state.schedule?.id
        || state.sourceSchedule?.id
        || null;
};

const finishDrag = () => {
    if (typeof scheduleDrag.endDrag === 'function') {
        scheduleDrag.endDrag();
    } else if (typeof scheduleDrag.clearDrag === 'function') {
        scheduleDrag.clearDrag();
    }
};

const issuesForSchedule = (scheduleId) => dataStore.issuesByScheduleId.get(scheduleId) || [];
const isScheduleFocused = (scheduleId) => focusedScheduleId.value === scheduleId;

const handlePointerDragStart = ({ schedule, event }) => {
    event?.preventDefault?.();
    if (typeof scheduleDrag.startDrag === 'function') {
        scheduleDrag.startDrag({
            schedule,
            event,
            source: cellTarget.value,
        });
    }
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

const handleEdit = (schedule) => {
    isEditMode.value = true;
    formValue.value = { ...schedule };
    showModal.value = true;
};

const handleDelete = (schedule) => {
    dialog.warning({
        title: '确认删除排课',
        content: `确定要删除这个排课记录吗？`,
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: () => {
            dataStore.deleteSchedule(schedule.id);
            message.success('排课已删除');
        },
    });
};

const handleLockToggle = (schedule) => {
    dataStore.setScheduleLocked(schedule.id, !schedule.is_locked);
};

const handleStage = (schedule) => {
    dataStore.stageSchedule(schedule.id);
    message.success('排课已暂存');
};


const handleBlockToggle = () => {
    dataStore.toggleUnavailableSlot(props.teacherId, props.dayId, props.timeId);
};

const openConflictChoice = (sourceScheduleId, targetScheduleId) => {
    if (!sourceScheduleId || !targetScheduleId || sourceScheduleId === targetScheduleId) {
        finishDrag();
        return;
    }

    conflictChoice.value = { sourceScheduleId, targetScheduleId };
    showConflictModal.value = true;
    finishDrag();
};

const installDraggedSchedule = (sourceScheduleId) => {
    if (!sourceScheduleId) return;
    dataStore.installSchedule(sourceScheduleId, cellTarget.value);
    finishDrag();
};

const resolveTargetScheduleId = (targetScheduleId) => {
    return targetScheduleId || schedulesForCell.value[0]?.id || null;
};

const handleBlankAreaDrop = () => {
    const sourceScheduleId = getDraggedScheduleId();
    if (!sourceScheduleId || isBlocked.value) return;

    if (hasSchedules.value) {
        openConflictChoice(sourceScheduleId, resolveTargetScheduleId());
        return;
    }

    installDraggedSchedule(sourceScheduleId);
};

const handleCardDrop = ({ targetScheduleId }) => {
    const sourceScheduleId = getDraggedScheduleId();
    if (!sourceScheduleId || isBlocked.value) return;
    openConflictChoice(sourceScheduleId, resolveTargetScheduleId(targetScheduleId));
};

const conflictTargetPlacement = () => ({
    teacher_id: props.teacherId,
    day_id: props.dayId,
    time_id: props.timeId,
});

const handleConflictReplace = () => {
    dataStore.replaceSchedule(conflictChoice.value.sourceScheduleId, conflictChoice.value.targetScheduleId, conflictTargetPlacement());
    showConflictModal.value = false;
};

const handleConflictSwap = () => {
    dataStore.swapSchedules(
        conflictChoice.value.sourceScheduleId,
        conflictChoice.value.targetScheduleId,
        ['teacher_id', 'day_id', 'time_id']
    );
    showConflictModal.value = false;
};

const handleConflictDisplace = () => {
    dataStore.displaceSchedule(conflictChoice.value.sourceScheduleId, conflictChoice.value.targetScheduleId, conflictTargetPlacement());
    showConflictModal.value = false;
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
.add-schedule {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 38px;
}

.blocked-schedule {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 44px;
    min-width: 88px;
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

.cell-issue-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    color: #fff;
    background: #d03050;
    border-radius: 999px;
    font-size: 12px;
    line-height: 1;
}

.conflict-choice {
    margin: 8px 0;
}
</style>
