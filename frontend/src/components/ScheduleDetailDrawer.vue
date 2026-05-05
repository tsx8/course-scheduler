<template>
    <n-drawer :show="show" :width="440" placement="right" @update:show="emit('update:show', $event)">
        <n-drawer-content :title="title" closable>
            <n-space vertical size="large">
                <section class="schedule-detail-drawer__section">
                    <h3>异常与风险</h3>
                    <n-empty v-if="!issues.length" description="当前范围没有异常或风险" size="small" />
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
import { NButton, NDrawer, NDrawerContent, NEmpty, NList, NListItem, NSpace } from 'naive-ui';
import ScheduleIssueTag from './ScheduleIssueTag.vue';

defineProps({
    show: { type: Boolean, default: false },
    title: { type: String, default: '排课详情' },
    issues: { type: Array, default: () => [] },
});

const emit = defineEmits(['update:show', 'locate']);
</script>

<style scoped>
.schedule-detail-drawer__section h3 {
    margin: 0 0 10px;
    color: #303133;
    font-size: 15px;
}


.schedule-detail-drawer__issue-message {
    color: #303133;
    line-height: 1.5;
}
</style>
