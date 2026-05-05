<script>
const hardSeverities = new Set([
    'error',
    'teacher_time_conflict',
    'teacher_unavailable_conflict',
    'teacher_course_mismatch',
    'teacher_campus_mismatch',
    'course_venue_mismatch',
    'teacher_day_campus_conflict'
]);

const warningSeverities = new Set([
    'warning',
    'capacity_risk',
    'teacher_hours_risk'
]);


export function getIssueSeverityMeta(severity) {
    if (hardSeverities.has(severity)) {
        return { group: 'hard', label: '硬冲突', tagType: 'error', className: 'severity-hard' };
    }
    if (warningSeverities.has(severity)) {
        return { group: 'warning', label: '风险', tagType: 'warning', className: 'severity-warning' };
    }
    return { group: 'warning', label: '风险', tagType: 'warning', className: 'severity-warning' };
}
</script>

<script setup>
import { computed } from 'vue';
import { NTag } from 'naive-ui';

const props = defineProps({
    severity: { type: String, required: true },
    text: { type: String, default: '' }
});

const meta = computed(() => getIssueSeverityMeta(props.severity));
const displayText = computed(() => props.text || meta.value.label);
</script>

<template>
    <n-tag size="small" :type="meta.tagType" :class="meta.className" round>
        {{ displayText }}
    </n-tag>
</template>

<style scoped>
.severity-hard {
    color: #d03050;
    background-color: #fff1f0;
    border-color: #ffa39e;
}

.severity-warning {
    color: #d46b08;
    background-color: #fff7e6;
    border-color: #ffd591;
}
</style>
