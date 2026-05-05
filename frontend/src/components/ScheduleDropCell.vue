<template>
    <div
        class="schedule-drop-cell"
        :class="cellClasses"
        tabindex="0"
        data-drop-cell="true"
        :data-target-type="targetData.type"
        :data-teacher-id="targetData.teacherId"
        :data-campus-id="targetData.campusId"
        :data-venue-id="targetData.venueId"
        :data-day-id="targetData.dayId"
        :data-time-id="targetData.timeId"
        @pointerenter="emitPointer('pointer-enter', $event)"
        @pointerleave="emitPointer('pointer-leave', $event)"
        @pointermove="emitPointer('pointer-move', $event)"
        @pointerdown="emitPointer('pointer-down', $event)"
        @pointerup="handlePointerUp"
    >
        <div
            v-if="hasIssueBadge"
            class="schedule-drop-cell__issue-badge"
            @click.stop="emit('issue-click', { target: props.target, issues })"
        >
            <slot name="issue-badge" :issues="issues" :count="issues.length">
                <n-badge :value="issues.length" :type="badgeType" :max="99" />
            </slot>
        </div>

        <div class="schedule-drop-cell__content">
            <slot />
        </div>

        <div v-if="showBlankDropArea" class="schedule-drop-cell__blank-area">
            <slot name="blank">
                <span>拖入排课</span>
            </slot>
        </div>
    </div>
</template>

<script setup>
import { computed, useSlots } from 'vue';
import { NBadge } from 'naive-ui';

const props = defineProps({
    target: { type: Object, default: () => ({}) },
    issues: { type: Array, default: () => [] },
    hover: { type: Boolean, default: false },
    focused: { type: Boolean, default: false },
    canDrop: { type: Boolean, default: true },
    disabled: { type: Boolean, default: false },
    occupied: { type: Boolean, default: false },
    showIssueBadge: { type: Boolean, default: true },
    showBlankArea: { type: Boolean, default: true },
});

const emit = defineEmits([
    'blank-area-drop',
    'card-drop',
    'pointer-enter',
    'pointer-leave',
    'pointer-move',
    'pointer-down',
    'pointer-up',
    'issue-click',
]);

const slots = useSlots();

const hasIssues = computed(() => props.issues.length > 0);
const hasIssueBadge = computed(() => props.showIssueBadge && (hasIssues.value || Boolean(slots['issue-badge'])));
const showBlankDropArea = computed(() => props.showBlankArea && !props.occupied && !props.disabled);
const highestSeverity = computed(() => {
    if (props.issues.some(issue => issue.severity === 'error')) return 'error';
    if (props.issues.some(issue => issue.severity === 'warning')) return 'warning';
    return 'default';
});
const badgeType = computed(() => (highestSeverity.value === 'default' ? 'default' : highestSeverity.value));
const targetData = computed(() => ({
    type: props.target?.type,
    teacherId: props.target?.teacher_id,
    campusId: props.target?.campus_id,
    venueId: props.target?.venue_id,
    dayId: props.target?.day_id,
    timeId: props.target?.time_id,
}));
const cellClasses = computed(() => ({
    'schedule-drop-cell--hover': props.hover,
    'schedule-drop-cell--focused': props.focused,
    'schedule-drop-cell--can-drop': props.canDrop && !props.disabled,
    'schedule-drop-cell--disabled': props.disabled,
    'schedule-drop-cell--occupied': props.occupied,
    'schedule-drop-cell--with-issues': hasIssues.value,
    [`schedule-drop-cell--severity-${highestSeverity.value}`]: hasIssues.value,
}));

const eventPayload = (event) => ({
    target: props.target,
    event,
});

const emitPointer = (name, event) => {
    emit(name, eventPayload(event));
};

const handlePointerUp = (event) => {
    const payload = eventPayload(event);
    emit('pointer-up', payload);

    if (props.disabled || !props.canDrop) return;

    if (event.target === event.currentTarget || event.target.closest?.('.schedule-drop-cell__blank-area')) {
        emit('blank-area-drop', payload);
        return;
    }

    const scheduleElement = event.target.closest?.('[data-schedule-id]');
    if (!scheduleElement) {
        emit('blank-area-drop', payload);
        return;
    }

    emit('card-drop', { ...payload, targetScheduleId: scheduleElement.dataset.scheduleId });
};
</script>

<style scoped>
.schedule-drop-cell {
    position: relative;
    width: 100%;
    min-width: 0;
    min-height: 64px;
    max-height: none;
    padding: 5px;
    box-sizing: border-box;
    overflow: visible;
    border: 1px dashed transparent;
    border-radius: 6px;
    background: #fff;
    transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;
}
.schedule-drop-cell:focus-visible {
    outline: 2px solid #18a058;
    outline-offset: 2px;
}


.schedule-drop-cell--can-drop {
    border-color: #d9d9d9;
}

.schedule-drop-cell--hover {
    background: #f0faff;
    border-color: #36ad6a;
}

.schedule-drop-cell--focused {
    z-index: 2;
}

.schedule-drop-cell--focused::after {
    content: '';
    position: absolute;
    inset: -2px;
    z-index: 3;
    pointer-events: none;
    border: 2px solid rgba(24, 160, 88, 0.82);
    border-radius: 8px;
    box-shadow: 0 0 0 3px rgba(24, 160, 88, 0.16);
    animation: schedule-drop-cell-locate-overlay 6s linear forwards;
}

@keyframes schedule-drop-cell-locate-overlay {
    0%, 83.333% {
        opacity: 1;
    }
    100% {
        opacity: 0;
    }
}

.schedule-drop-cell--disabled {
    background: #f5f5f5;
    cursor: not-allowed;
}

.schedule-drop-cell--with-issues {
    border-color: #ffccc7;
}
.schedule-drop-cell--severity-error {
    border-color: #ffa39e;
    background: #fffafa;
}

.schedule-drop-cell--severity-warning {
    border-color: #ffd591;
    background: #fffdf6;
}



.schedule-drop-cell__issue-badge {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 1;
    cursor: pointer;
}

.schedule-drop-cell__content {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    gap: 5px;
    overflow: visible;
}

.schedule-drop-cell__blank-area {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 0;
    padding: 0;
    color: #909399;
    font-size: 12px;
}

.schedule-drop-cell--hover .schedule-drop-cell__blank-area {
    color: #18a058;
}
</style>
