<template>
    <article
        class="schedule-card"
        :class="cardClasses"
        tabindex="0"
        role="button"
        @pointerdown="handlePointerDragStart"
        :data-schedule-id="schedule?.id"
        @pointerup="handlePointerDrop"
    >
        <div class="schedule-card__header">
            <div class="schedule-card__title">
                <strong>{{ courseName }}</strong>
                <span v-if="contextSubtitle" class="schedule-card__subtitle">{{ contextSubtitle }}</span>
                <span v-if="schedule?.is_staged" class="schedule-card__staged">暂存</span>
            </div>
            <div v-if="showBadges" class="schedule-card__badges">
                <n-tag :type="schedule?.is_locked ? 'default' : 'success'" size="small" round>
                    {{ schedule?.is_locked ? '已锁定' : '未锁定' }}
                </n-tag>
                <slot name="issue-tags" :issues="issues">
                    <ScheduleIssueTag
                        v-if="primaryIssue"
                        :severity="primaryIssue.severity"
                    />
                    <span v-if="extraIssueCount" class="schedule-card__issue-count">+{{ extraIssueCount }}</span>
                </slot>
            </div>
        </div>

        <div v-if="isStagingDensity" class="schedule-card__meta">
            <span>{{ teacherName }} · {{ dayName }} {{ timeName }}</span>
            <span>{{ campusName }} · {{ venueName }}</span>
        </div>

        <dl v-else-if="showDetailGrid" class="schedule-card__details">
            <div v-if="showTeacher" class="schedule-card__detail">
                <dt>教师</dt>
                <dd>{{ teacherName }}</dd>
            </div>
            <div v-if="showDateTime" class="schedule-card__detail">
                <dt>日期</dt>
                <dd>{{ dayName }}</dd>
            </div>
            <div v-if="showDateTime" class="schedule-card__detail">
                <dt>时间</dt>
                <dd>{{ timeName }}</dd>
            </div>
            <div v-if="showCampus" class="schedule-card__detail">
                <dt>校区</dt>
                <dd>{{ campusName }}</dd>
            </div>
            <div class="schedule-card__detail">
                <dt>场地</dt>
                <dd>{{ venueName }}</dd>
            </div>
        </dl>

        <div v-if="hasVisibleActions" class="schedule-card__actions">
            <n-button v-if="actionConfig.viewDetails" quaternary size="tiny" :aria-label="'详情'" title="详情" @pointerdown.stop @click.stop="emitAction('view-details')">
                <n-icon v-if="isCompactDensity" :component="DetailsIcon" />
                <template v-else>详情</template>
            </n-button>
            <n-button v-if="actionConfig.edit" quaternary size="tiny" :aria-label="'编辑'" title="编辑" @pointerdown.stop @click.stop="emitAction('edit')">
                <n-icon v-if="isCompactDensity" :component="EditIcon" />
                <template v-else>编辑</template>
            </n-button>
            <n-button v-if="actionConfig.lock" class="schedule-card__lock-action" :class="schedule?.is_locked ? 'schedule-card__lock-action--locked' : 'schedule-card__lock-action--unlocked'" quaternary size="tiny" @pointerdown.stop @click.stop="emitAction('lock-toggle')" :aria-label="schedule?.is_locked ? '取消锁定' : '锁定课程'" :title="lockActionText">
                <n-icon v-if="isCompactDensity" :component="lockIcon" />
                <template v-else>{{ lockActionText }}</template>
            </n-button>
            <n-button v-if="actionConfig.stage" quaternary size="tiny" :aria-label="'暂存'" title="暂存" :disabled="schedule?.is_staged" @pointerdown.stop @click.stop="emitAction('stage')">
                <n-icon v-if="isCompactDensity" :component="StageIcon" />
                <template v-else>暂存</template>
            </n-button>
            <n-button v-if="actionConfig.delete" quaternary size="tiny" type="error" :aria-label="'删除'" title="删除" @pointerdown.stop @click.stop="emitAction('delete')">
                <n-icon v-if="isCompactDensity" :component="DeleteIcon" />
                <template v-else>删除</template>
            </n-button>
        </div>
    </article>
</template>

<script setup>
import { computed } from 'vue';
import { NButton, NIcon, NTag } from 'naive-ui';
import { ArchiveOutline as StageIcon, CreateOutline as EditIcon, InformationCircleOutline as DetailsIcon, LockClosedOutline, LockOpenOutline, TrashOutline as DeleteIcon } from '@vicons/ionicons5';
import { useDataStore } from '../stores/data';
import ScheduleIssueTag from './ScheduleIssueTag.vue';

const props = defineProps({
    schedule: { type: Object, required: true },
    context: {
        type: String,
        default: 'teacher',
        validator: value => ['teacher', 'campus', 'staging'].includes(value),
    },
    issues: { type: Array, default: () => [] },
    focused: { type: Boolean, default: false },
    actions: { type: Object, default: () => ({}) },
    density: {
        type: String,
        default: '',
        validator: value => ['', 'compact', 'normal', 'staging'].includes(value),
    },
});

const emit = defineEmits([
    'lock-toggle',
    'stage',
    'edit',
    'delete',
    'view-details',
    'pointer-drag-start',
    'pointer-drop',
]);

const dataStore = useDataStore();

const findById = (records, id) => records.find(record => record.id === id);

const teacherName = computed(() => findById(dataStore.teachers, props.schedule?.teacher_id)?.name || '未知教师');
const courseName = computed(() => findById(dataStore.courses, props.schedule?.course_id)?.name || '未知课程');
const campusName = computed(() => findById(dataStore.campuses, props.schedule?.campus_id)?.name || '未知校区');
const venueName = computed(() => findById(dataStore.venues, props.schedule?.venue_id)?.name || '未知场地');
const dayName = computed(() => findById(dataStore.day, props.schedule?.day_id)?.value || '未知日期');
const timeName = computed(() => findById(dataStore.time, props.schedule?.time_id)?.value || '未知时间');

const showTeacher = computed(() => props.context !== 'teacher');
const showCampus = computed(() => props.context !== 'campus');
const showDateTime = computed(() => props.context === 'staging');
const contextSubtitle = computed(() => {
    if (props.context === 'teacher') return `${campusName.value} - ${venueName.value}`;
    if (props.context === 'campus') return `${teacherName.value} - ${venueName.value}`;
    return '';
});
const effectiveDensity = computed(() => {
    if (props.density) return props.density;
    return props.context === 'staging' ? 'staging' : 'compact';
});
const isCompactDensity = computed(() => effectiveDensity.value === 'compact');
const isStagingDensity = computed(() => effectiveDensity.value === 'staging');
const showDetailGrid = computed(() => effectiveDensity.value === 'normal');
const severityRank = { error: 0, warning: 1, info: 2 };
const visibleIssues = computed(() => props.issues.filter(issue => issue));
const orderedIssues = computed(() => visibleIssues.value.slice().sort((left, right) => {
    const leftRank = severityRank[left.severity] ?? 3;
    const rightRank = severityRank[right.severity] ?? 3;
    return leftRank - rightRank;
}));
const primaryIssue = computed(() => orderedIssues.value[0] || null);
const extraIssueCount = computed(() => Math.max(0, visibleIssues.value.length - 1));
const lockActionText = computed(() => {
    if (isStagingDensity.value) return props.schedule?.is_locked ? '解锁' : '锁定';
    return props.schedule?.is_locked ? '取消锁定' : '锁定课程';
});
const lockIcon = computed(() => (props.schedule?.is_locked ? LockClosedOutline : LockOpenOutline));
const defaultActions = {
    viewDetails: true,
    edit: true,
    lock: true,
    stage: true,
    delete: true,
};
const actionConfig = computed(() => ({ ...defaultActions, ...(props.actions || {}) }));
const cardClasses = computed(() => ({
    'schedule-card--focused': props.focused,
    'schedule-card--locked': props.schedule?.is_locked,
    'schedule-card--unlocked': props.schedule?.is_locked === false,
    'schedule-card--staged': props.schedule?.is_staged,
    [`schedule-card--${props.context}`]: true,
    [`schedule-card--density-${effectiveDensity.value}`]: true,
}));
const showBadges = computed(() => effectiveDensity.value !== 'compact');
const hasVisibleActions = computed(() => Object.values(actionConfig.value).some(Boolean));

const emitAction = (eventName) => {
    emit(eventName, props.schedule);
};

const releasePointerCapture = (target, pointerId) => {
    if (!target?.releasePointerCapture || pointerId === undefined) return;

    try {
        target.releasePointerCapture(pointerId);
    } catch (error) {
        void error;
    }
};

const handlePointerDragStart = (event) => {
    const pointerTarget = event.currentTarget;
    const pointerId = event.pointerId;
    releasePointerCapture(pointerTarget, pointerId);
    window.setTimeout(() => releasePointerCapture(pointerTarget, pointerId), 0);
    emit('pointer-drag-start', { schedule: props.schedule, event });
};
const handlePointerDrop = (event) => {
    emit('pointer-drop', { schedule: props.schedule, event });
};

</script>

<style scoped>
.schedule-card {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 0 0 auto;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    box-sizing: border-box;
    min-height: 0;
    gap: 6px;
    padding: 6px 8px;
    overflow: hidden;
    border: 1px solid #91d5ff;
    border-radius: 6px;
    background: #e6f7ff;
    text-align: left;
    cursor: grab;
    transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;
    touch-action: none;
}

.schedule-card:focus-visible {
    outline: 2px solid #18a058;
    outline-offset: 2px;
}

.schedule-card:active {
    cursor: grabbing;
}

.schedule-card--focused::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    border: 2px solid rgba(24, 160, 88, 0.82);
    border-radius: inherit;
    box-shadow: inset 0 0 0 2px rgba(24, 160, 88, 0.16);
    animation: schedule-card-locate-overlay 6s linear forwards;
}


@keyframes schedule-card-locate-overlay {
    0%, 83.333% {
        opacity: 1;
    }
    100% {
        opacity: 0;
    }
}

.schedule-card--locked {
    border-color: #60a5fa;
}

.schedule-card--unlocked {
    border-style: dashed;
    border-color: #36ad6a;
    background: #f0fff4;
}

.schedule-card--staged {
    background: #fff7e6;
    border-color: #ffd591;
}

.schedule-card--density-compact {
    gap: 3px;
    padding: 5px 7px;
}

.schedule-card--density-staging {
    height: 100%;
    gap: 4px;
    padding: 6px 8px;
    overflow: auto;
}

.schedule-card__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    min-width: 0;
    gap: 6px;
}

.schedule-card__title {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
}

.schedule-card__title strong,
.schedule-card__subtitle,
.schedule-card__staged,
.schedule-card__meta span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.schedule-card__title strong {
    color: #1f2937;
    font-size: 13px;
    line-height: 1.25;
}

.schedule-card__subtitle {
    color: #1f2937;
}

.schedule-card__staged {
    color: #d46b08;
}

.schedule-card__subtitle,
.schedule-card__staged {
    font-size: 12px;
    line-height: 1.2;
}

.schedule-card--density-compact .schedule-card__header {
    align-items: center;
}

.schedule-card--density-compact .schedule-card__badges {
    display: none;
}

.schedule-card__badges {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    max-width: 46%;
    overflow: hidden;
}

.schedule-card__badges :deep(.n-tag) {
    max-width: 72px;
    overflow: hidden;
}

.schedule-card__issue-count {
    flex: 0 0 auto;
    color: #64748b;
    font-size: 12px;
    line-height: 1;
}

.schedule-card__meta {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    gap: 1px;
    overflow: hidden;
    color: #4b5563;
    font-size: 12px;
    line-height: 1.25;
}

.schedule-card__details {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px 10px;
    margin: 0;
}

.schedule-card__detail {
    min-width: 0;
}

.schedule-card__detail dt {
    color: #606266;
    font-size: 12px;
}

.schedule-card__detail dd {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.schedule-card__actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 2px;
    min-width: 0;
    max-height: 44px;
    overflow: hidden;
    opacity: 0.68;
    transition: opacity 0.2s;
}

.schedule-card--density-compact .schedule-card__actions {
    align-self: flex-end;
    flex-wrap: nowrap;
    justify-content: flex-end;
    gap: 2px;
    max-height: none;
    overflow: visible;
}

.schedule-card--density-staging .schedule-card__actions {
    margin-top: auto;
    max-height: none;
    flex-wrap: wrap;
    overflow: visible;
}

.schedule-card__actions :deep(.n-button) {
    --n-padding: 0 4px;
    font-size: 12px;
}
.schedule-card--density-compact .schedule-card__actions :deep(.n-button) {
    width: 20px;
    min-width: 20px;
    height: 22px;
    --n-padding: 0;
}

.schedule-card__lock-action--locked {
    color: #2563eb;
}

.schedule-card__lock-action--unlocked {
    color: #18a058;
}

.schedule-card__lock-action :deep(.n-button__icon) {
    opacity: 1;
}


.schedule-card--density-staging .schedule-card__actions :deep(.n-button) {
    height: 20px;
    min-width: 0;
}

.schedule-card:hover .schedule-card__actions,
.schedule-card:focus-within .schedule-card__actions {
    opacity: 1;
}
</style>
