<template>
    <section
        v-if="visible"
        class="schedule-staging-tray"
        :class="{ 'schedule-staging-tray--dragging': dragStore.isDragging }"
        role="region"
        aria-label="暂存区"
        @pointerenter="handlePointerEnter"
        @pointerleave="handlePointerLeave"
        @pointerup="handleTrayDrop"
    >
        <div class="schedule-staging-tray__header">
            <div>
                <strong>暂存区</strong>
                <span class="schedule-staging-tray__count">{{ stagedSchedules.length }} 项</span>
            </div>
            <n-button
                v-if="stagedSchedules.length"
                size="tiny"
                quaternary
                type="error"
                aria-label="清空暂存区"
                @pointerdown.stop
                @click.stop="clearStagedSchedules"
            >
                清空
            </n-button>
        </div>

        <div class="schedule-staging-tray__scroll" aria-label="暂存课程列表">
            <ScheduleCard
                v-for="schedule in stagedSchedules"
                :key="schedule.id"
                :schedule="schedule"
                context="staging"
                :issues="dataStore.issuesByScheduleId.get(schedule.id) || []"
                :actions="{ edit: false, stage: false }"
                @pointer-drag-start="handleCardDragStart"
                @lock-toggle="handleLockToggle"
                @delete="handleDelete"
            />
            <div
                class="schedule-staging-tray__drop-slot"
                :class="{ 'schedule-staging-tray__drop-slot--active': dragStore.isDragging }"
                role="button"
                tabindex="0"
                aria-label="拖到这里暂存课程"
                @pointerup.stop="handleDropSlotDrop"
            >
                拖到这里暂存
            </div>
        </div>
    </section>
</template>

<script setup>
import { computed } from 'vue';
import { NButton } from 'naive-ui';
import { useDataStore } from '../stores/data';
import { useScheduleDragStore } from '../stores/scheduleDrag';
import ScheduleCard from './ScheduleCard.vue';

defineProps({
    visible: { type: Boolean, default: false },
});

const dataStore = useDataStore();
const dragStore = useScheduleDragStore();

const stagedSchedules = computed(() => dataStore.stagedScheduledClasses);

const handleCardDragStart = ({ schedule, event }) => {
    dragStore.startDrag({ schedule, source: { type: 'staging' }, event });
};

const handleLockToggle = (schedule) => {
    dataStore.setScheduleLocked(schedule.id, !schedule.is_locked);
};

const handleDelete = (schedule) => {
    dataStore.deleteSchedule(schedule.id);
};

const stageDraggedSchedule = () => {
    const scheduleId = dragStore.draggedScheduleId;
    if (!scheduleId) return;

    const schedule = dataStore.scheduledClasses.find(item => item.id === scheduleId);
    if (schedule && !schedule.is_staged) {
        dataStore.stageSchedule(scheduleId);
    }
    dragStore.endDrag();
};

const handleDropSlotDrop = () => {
    stageDraggedSchedule();
};

const handleTrayDrop = (event) => {
    if (!event.target.closest?.('.schedule-staging-tray__drop-slot')) {
        stageDraggedSchedule();
    }
};

const handlePointerEnter = () => {
    dragStore.setHoverTarget({ type: 'staging' });
};

const handlePointerLeave = () => {
    if (dragStore.hoverTarget?.type === 'staging') {
        dragStore.setHoverTarget(null);
    }
};

const clearStagedSchedules = () => {
    if (!window.confirm('确定清空暂存区吗？暂存的排课将被删除。')) return;

    dataStore.stagedScheduledClasses.forEach(schedule => {
        dataStore.deleteSchedule(schedule.id);
    });
    dragStore.cancelDrag();
};
</script>

<style scoped>
.schedule-staging-tray {
    flex: 0 0 148px;
    height: 148px;
    min-height: 0;
    margin-top: 12px;
    padding: 8px 10px 10px;
    overflow: hidden;
    border: 1px solid #ffd591;
    border-radius: 8px;
    background: #fffaf0;
    box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.04);
    --wails-draggable: no-drag;
}

.schedule-staging-tray--dragging {
    border-color: #18a058;
    background: #f6ffed;
}

.schedule-staging-tray__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: 24px;
    margin-bottom: 6px;
}

.schedule-staging-tray__count {
    margin-left: 8px;
    color: #909399;
    font-size: 12px;
}

.schedule-staging-tray__scroll {
    display: flex;
    height: calc(100% - 30px);
    min-height: 0;
    gap: 8px;
    overflow-x: auto;
    overflow-y: hidden;
    padding-bottom: 2px;
}

.schedule-staging-tray__scroll :deep(.schedule-card) {
    flex: 0 0 204px;
    height: 100%;
}

.schedule-staging-tray__drop-slot {
    display: flex;
    flex: 0 0 160px;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 0;
    border: 1px dashed #d9d9d9;
    border-radius: 6px;
    color: #909399;
    background: #fff;
    font-size: 13px;
    cursor: default;
    transition: border-color 0.2s, color 0.2s, background-color 0.2s;
}

.schedule-staging-tray__drop-slot--active {
    border-color: #18a058;
    color: #18a058;
    background: #f6ffed;
}
</style>
