<template>
    <ScheduleDropCell
        :target="cellTarget"
        :issues="cellIssues"
        :hover="isCellDropTargeted"
        :focused="isCellFocused"
        :can-drop="!isBlocked"
        :occupied="hasSchedules"
        :show-issue-badge="false"
        @pointer-enter="handleCellPointerMove"
        @pointer-move="handleCellPointerMove"
        @pointer-leave="handleCellPointerLeave"
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
                :drop-targeted="isScheduleDropTargeted(schedule.id)"
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

    <ScheduleEditModal
        v-model:show="showModal"
        mode="teacher"
        :teacher-id="teacherId"
        :day-id="dayId"
        :time-id="timeId"
        :schedule="editingSchedule"
    />

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
    NButton, NButtonGroup, NIcon, NModal, useMessage, useDialog
} from 'naive-ui';
import {
    AddOutline as AddIcon,
    BanOutline as BlockIcon,
} from '@vicons/ionicons5';
import ScheduleCard from './ScheduleCard.vue';
import ScheduleDropCell from './ScheduleDropCell.vue';
import ScheduleEditModal from './ScheduleEditModal.vue';

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
const editingSchedule = ref(null);
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
const hoverTarget = computed(() => scheduleDrag.hoverTarget || null);
const isCellFocused = computed(() => {
    const target = focusedTarget.value;
    if (!target) return false;
    return (!target.type || target.type === 'teacher')
        && target.teacher_id === props.teacherId
        && target.day_id === props.dayId
        && target.time_id === props.timeId;
});

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

const hoverMatchesTeacherCell = (target) => {
    return target?.type === 'teacher'
        && target.teacher_id === props.teacherId
        && target.day_id === props.dayId
        && target.time_id === props.timeId;
};

const isCellDropTargeted = computed(() => {
    return hoverMatchesTeacherCell(hoverTarget.value)
        && (hoverTarget.value.drop_mode ?? 'cell') === 'cell';
});

const isScheduleDropTargeted = (scheduleId) => {
    return hoverMatchesTeacherCell(hoverTarget.value)
        && hoverTarget.value.drop_mode === 'schedule'
        && hoverTarget.value.target_schedule_id === scheduleId;
};

const setCellHoverTarget = (target) => {
    scheduleDrag.setHoverTarget({ ...target, drop_mode: 'cell' });
};

const setScheduleHoverTarget = (target, targetScheduleId) => {
    scheduleDrag.setHoverTarget({
        ...target,
        drop_mode: 'schedule',
        target_schedule_id: targetScheduleId,
    });
};

const handleCellPointerMove = ({ target, event }) => {
    const sourceScheduleId = getDraggedScheduleId();
    if (!sourceScheduleId) return;

    const targetScheduleId = event.target.closest?.('[data-schedule-id]')?.dataset?.scheduleId;
    if (targetScheduleId && targetScheduleId !== sourceScheduleId) {
        setScheduleHoverTarget(target, targetScheduleId);
        return;
    }

    setCellHoverTarget(target);
};

const handleCellPointerLeave = () => {
    if (!getDraggedScheduleId() || !hoverMatchesTeacherCell(hoverTarget.value)) return;
    scheduleDrag.setHoverTarget(null);
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
    editingSchedule.value = null;
    showModal.value = true;
};

const handleEdit = (schedule) => {
    editingSchedule.value = { ...schedule };
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
