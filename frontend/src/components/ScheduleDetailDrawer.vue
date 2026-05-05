<template>
    <n-drawer :show="show" :width="440" placement="right" @update:show="emit('update:show', $event)">
        <n-drawer-content :title="title" closable>
            <n-space vertical size="large">
                <section v-if="summaryItems.length" class="schedule-detail-drawer__section">
                    <h3>概览</h3>
                    <div class="schedule-detail-drawer__summary">
                        <div
                            v-for="item in summaryItems"
                            :key="item.label"
                            class="schedule-detail-drawer__summary-item"
                            :class="item.type ? `schedule-detail-drawer__summary-item--${item.type}` : null"
                        >
                            <span>{{ item.label }}</span>
                            <strong>{{ item.value }}</strong>
                        </div>
                    </div>
                </section>

                <section class="schedule-detail-drawer__section">
                    <h3>排课</h3>
                    <n-empty v-if="!schedules.length" :description="emptyScheduleText" size="small" />
                    <n-list v-else bordered>
                        <n-list-item v-for="schedule in schedules" :key="schedule.id">
                            <n-thing :title="courseName(schedule)">
                                <template #description>
                                    <n-space size="small">
                                        <n-tag size="small" :type="schedule.is_locked ? 'default' : 'success'" round>
                                            {{ schedule.is_locked ? '已锁定' : '可调整' }}
                                        </n-tag>
                                        <n-tag v-if="schedule.is_staged" size="small" type="warning" round>暂存</n-tag>
                                    </n-space>
                                </template>
                                <div class="schedule-detail-drawer__line">
                                    {{ teacherName(schedule) }} · {{ campusName(schedule) }} · {{ venueName(schedule) }}
                                </div>
                                <div class="schedule-detail-drawer__line">
                                    {{ dayName(schedule) }} {{ timeName(schedule) }}
                                </div>
                            </n-thing>
                        </n-list-item>
                    </n-list>
                </section>

                <section class="schedule-detail-drawer__section">
                    <h3>异常与提示</h3>
                    <n-empty v-if="!issues.length" description="当前范围没有异常或提示" size="small" />
                    <n-list v-else bordered>
                        <n-list-item v-for="issue in issues" :key="issue.id">
                            <n-space vertical size="small">
                                <n-space align="center" :wrap="false">
                                    <ScheduleIssueTag :severity="issue.severity" />
                                    <span class="schedule-detail-drawer__issue-message">{{ issue.message }}</span>
                                </n-space>
                                <n-button
                                    v-if="issue.focus"
                                    size="tiny"
                                    text
                                    type="primary"
                                    @click="emit('locate', issue)"
                                >
                                    定位
                                </n-button>
                            </n-space>
                        </n-list-item>
                    </n-list>
                </section>
            </n-space>
        </n-drawer-content>
    </n-drawer>
</template>

<script setup>
import { NButton, NDrawer, NDrawerContent, NEmpty, NList, NListItem, NSpace, NTag, NThing } from 'naive-ui';
import { useDataStore } from '../stores/data';
import ScheduleIssueTag from './ScheduleIssueTag.vue';

const props = defineProps({
    show: { type: Boolean, default: false },
    title: { type: String, default: '排课详情' },
    schedules: { type: Array, default: () => [] },
    issues: { type: Array, default: () => [] },
    summaryItems: { type: Array, default: () => [] },
    emptyScheduleText: { type: String, default: '当前范围没有排课' },
});

const emit = defineEmits(['update:show', 'locate']);
const dataStore = useDataStore();

const findById = (records, id) => records.find(record => record.id === id);
const teacherName = (schedule) => findById(dataStore.teachers, schedule.teacher_id)?.name || '未知教师';
const courseName = (schedule) => findById(dataStore.courses, schedule.course_id)?.name || '未知课程';
const campusName = (schedule) => findById(dataStore.campuses, schedule.campus_id)?.name || '未知校区';
const venueName = (schedule) => findById(dataStore.venues, schedule.venue_id)?.name || '未知场地';
const dayName = (schedule) => findById(dataStore.day, schedule.day_id)?.value || '未知日期';
const timeName = (schedule) => findById(dataStore.time, schedule.time_id)?.value || '未知时间';
</script>

<style scoped>
.schedule-detail-drawer__section h3 {
    margin: 0 0 10px;
    color: #303133;
    font-size: 15px;
}

.schedule-detail-drawer__summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
}

.schedule-detail-drawer__summary-item {
    padding: 8px 10px;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    background: #f8fafc;
}

.schedule-detail-drawer__summary-item span {
    display: block;
    color: #606266;
    font-size: 12px;
}

.schedule-detail-drawer__summary-item strong {
    display: block;
    margin-top: 2px;
    color: #303133;
    font-size: 16px;
}

.schedule-detail-drawer__summary-item--error {
    border-color: #ffa39e;
    background: #fff1f0;
}

.schedule-detail-drawer__summary-item--warning {
    border-color: #ffd591;
    background: #fff7e6;
}

.schedule-detail-drawer__line {
    color: #606266;
    font-size: 13px;
    line-height: 1.7;
}

.schedule-detail-drawer__issue-message {
    color: #303133;
    line-height: 1.5;
}
</style>
