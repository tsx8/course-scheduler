import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

export const useScheduleDragStore = defineStore('scheduleDrag', () => {
    const draggedSchedule = ref(null);
    const sourceContext = ref(null);
    const hoverTarget = ref(null);
    const pointerId = ref(null);

    const isDragging = computed(() => Boolean(draggedSchedule.value));
    const draggedScheduleId = computed(() => draggedSchedule.value?.id ?? null);

    const startDrag = ({ schedule, source = null, event = null } = {}) => {
        if (!schedule?.id) return;

        draggedSchedule.value = schedule;
        sourceContext.value = source;
        hoverTarget.value = null;
        pointerId.value = event?.pointerId ?? null;
    };

    const setHoverTarget = (target) => {
        hoverTarget.value = target ? { ...target } : null;
    };

    const endDrag = () => {
        draggedSchedule.value = null;
        sourceContext.value = null;
        hoverTarget.value = null;
        pointerId.value = null;
    };

    const cancelDrag = () => {
        endDrag();
    };

    return {
        draggedSchedule,
        draggedScheduleId,
        sourceContext,
        hoverTarget,
        pointerId,
        isDragging,
        startDrag,
        setHoverTarget,
        endDrag,
        cancelDrag,
    };
});
