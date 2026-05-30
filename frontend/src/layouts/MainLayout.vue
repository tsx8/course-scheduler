<template>
    <n-spin class="app-spin" :show="dataStore.isSolving" size="large">
        <template #description>
            正在排课中……
        </template>
        <n-layout class="app-shell">
            <n-layout-header
                class="app-titlebar"
                bordered>
                <h1 style="font-size: 1.2em; margin: 0; color: #333; user-select: none; --wails-draggable: drag;">
                    排课管理系统
                </h1>
                <div class="app-titlebar__right">
                    <div v-if="dataStore.isInitialized" class="schedule-plan-switcher">
                        <span class="schedule-plan-switcher__label">课表版本</span>
                        <n-select
                            class="schedule-plan-switcher__select"
                            :value="dataStore.activeSchedulePlanId"
                            :options="dataStore.schedulePlanOptions"
                            size="small"
                            filterable
                            :disabled="schedulePlanActionPending"
                            @update:value="handleSchedulePlanSwitch"
                        />
                        <n-button-group>
                            <n-tooltip>
                                <template #trigger>
                                    <n-button size="small" :disabled="schedulePlanActionPending" @click="openSchedulePlanModal('create')">
                                        <template #icon><n-icon :component="AddIcon" /></template>
                                    </n-button>
                                </template>
                                新建空白课表版本
                            </n-tooltip>
                            <n-tooltip>
                                <template #trigger>
                                    <n-button size="small" :disabled="schedulePlanActionPending" @click="openSchedulePlanModal('copy')">
                                        <template #icon><n-icon :component="CopyIcon" /></template>
                                    </n-button>
                                </template>
                                复制当前课表版本
                            </n-tooltip>
                            <n-tooltip>
                                <template #trigger>
                                    <n-button size="small" :disabled="schedulePlanActionPending" @click="openSchedulePlanModal('rename')">
                                        <template #icon><n-icon :component="EditIcon" /></template>
                                    </n-button>
                                </template>
                                重命名当前课表版本
                            </n-tooltip>
                            <n-tooltip>
                                <template #trigger>
                                    <n-button size="small" :disabled="schedulePlanActionPending || dataStore.schedulePlans.length <= 1" @click="confirmDeleteSchedulePlan">
                                        <template #icon><n-icon :component="DeleteIcon" /></template>
                                    </n-button>
                                </template>
                                {{ dataStore.schedulePlans.length <= 1 ? '不能删除最后一个课表版本' : '删除当前课表版本' }}
                            </n-tooltip>
                        </n-button-group>
                    </div>
                    <n-button-group>
                        <n-button quaternary size="small" @click="minimizeWindow" title="最小化">
                            <template #icon><n-icon :component="MinimizeIcon" /></template>
                        </n-button>
                        <n-button quaternary size="small" @click="toggleMaximizeWindow" title="最大化/还原">
                            <template #icon>
                                <n-icon :component="isMaximized ? RestoreIcon : MaximizeIcon" /></template>
                        </n-button>
                        <n-button quaternary type="error" size="small" @click="appWindow.close()" title="关闭">
                            <template #icon><n-icon :component="CloseIcon" /></template>
                        </n-button>
                    </n-button-group>
                </div>
            </n-layout-header>
            <n-layout class="app-main" has-sider>
                <n-layout-sider bordered collapse-mode="width" show-trigger v-model:collapsed="collapsed">
                    <n-menu :options="menuOptions" :value="activeMenuKey" @update:value="handleMenuSelect" />
                </n-layout-sider>
                <n-layout-content
                    class="main-layout__content"
                    content-style="height: 100%; min-height: 0; padding: 24px; display: flex; flex-direction: column; overflow: hidden;"
                    :native-scrollbar="true"
                    @pointerdown.capture="handleSchedulePointerDown"
                    @pointerup="handleSchedulePointerEnd"
                    @pointercancel="handleSchedulePointerEnd">
                    <div class="main-layout__route" data-main-route>
                        <router-view v-if="dataStore.isInitialized" />
                        <div v-else class="main-layout__loading">
                            <n-spin size="large" />
                            <p style="margin-left: 12px;">正在加载数据...</p>
                        </div>
                    </div>
                    <ScheduleStagingTray v-if="dataStore.isInitialized" :visible="showStagingTray" />
                </n-layout-content>
            </n-layout>
        </n-layout>
        <n-modal v-model:show="schedulePlanModal.show" preset="dialog" :title="schedulePlanModalTitle" style="width: 420px;">
            <n-form ref="schedulePlanFormRef" :model="schedulePlanForm" :rules="schedulePlanRules" @submit.prevent="submitSchedulePlanModal">
                <n-form-item label="名称" path="name">
                    <n-input v-model:value="schedulePlanForm.name" placeholder="输入课表版本名称" @keydown.enter.prevent="submitSchedulePlanModal" />
                </n-form-item>
                <div class="schedule-plan-modal__actions">
                    <n-button :disabled="schedulePlanActionPending" @click="closeSchedulePlanModal">取消</n-button>
                    <n-button type="primary" :loading="schedulePlanActionPending" @click="submitSchedulePlanModal">
                        {{ schedulePlanSubmitText }}
                    </n-button>
                </div>
            </n-form>
        </n-modal>
    </n-spin>
</template>

<script setup>
import { h, ref, computed, onMounted, onUnmounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useDataStore } from '../stores/data';
import { useScheduleDragStore } from '../stores/scheduleDrag';
import ScheduleStagingTray from '../components/ScheduleStagingTray.vue';
import {
    NLayout, NLayoutHeader, NLayoutSider, NLayoutContent, NMenu, NIcon,
    NSpin, NButton, NButtonGroup, NSelect, NTooltip, NModal, NForm, NFormItem,
    NInput, useDialog, useMessage
} from 'naive-ui';
import {
    AddOutline as AddIcon,
    BookOutline as BookIcon,
    CopyOutline as CopyIcon,
    CreateOutline as EditIcon,
    TrashOutline as DeleteIcon,
    PeopleOutline as PeopleIcon,
    HomeOutline as HomeIcon,
    CalendarOutline as CalendarIcon,
    PersonOutline as PersonIcon,
    AlertCircleOutline as IssueIcon,
    RemoveOutline as MinimizeIcon,
    SquareOutline as MaximizeIcon,
    ResizeOutline as RestoreIcon,
    CloseSharp as CloseIcon,
    SettingsOutline as SettingsIcon,
    RocketOutline as SolverIcon,
    SaveOutline as SaveIcon,
    RefreshOutline as UndoIcon
} from '@vicons/ionicons5';
import { getCurrentWindow, invoke } from '../host/desktop';

const dataStore = useDataStore();
const dragStore = useScheduleDragStore();
const route = useRoute();
const router = useRouter();
const dialog = useDialog();
const message = useMessage();

const appWindow = getCurrentWindow();
const isMaximized = ref(false);
const isDialogVisible = ref(false);
let unlistenResize = null;
let unlistenClose = null;

const collapsed = ref(false);
const schedulePlanActionPending = ref(false);
const schedulePlanFormRef = ref(null);
const schedulePlanForm = ref({ name: '' });
const schedulePlanModal = ref({
    show: false,
    mode: 'create'
});

const schedulePlanRules = {
    name: { required: true, message: '请输入课表版本名称', trigger: ['input', 'blur'] }
};

const schedulePlanModalTitle = computed(() => {
    if (schedulePlanModal.value.mode === 'copy') return '复制课表版本';
    if (schedulePlanModal.value.mode === 'rename') return '重命名课表版本';
    return '新建课表版本';
});

const schedulePlanSubmitText = computed(() => {
    if (schedulePlanModal.value.mode === 'copy') return '复制';
    if (schedulePlanModal.value.mode === 'rename') return '保存';
    return '新建';
});

const schedulePlanBlockedMessage = '请先保存或撤销未保存更改，再管理课表版本。';

const errorMessage = (error) => error?.message || String(error);

const uniqueSchedulePlanName = (preferredName) => {
    const baseName = (preferredName || '新课表').trim() || '新课表';
    const existingNames = new Set(dataStore.schedulePlans.map(plan => plan.name));
    if (!existingNames.has(baseName)) return baseName;

    let suffix = 2;
    while (existingNames.has(`${baseName} ${suffix}`)) {
        suffix += 1;
    }
    return `${baseName} ${suffix}`;
};

const defaultSchedulePlanName = (mode) => {
    if (mode === 'copy') {
        return uniqueSchedulePlanName(`${dataStore.activeSchedulePlan?.name || '当前课表'} 副本`);
    }
    if (mode === 'rename') {
        return dataStore.activeSchedulePlan?.name || '';
    }
    return uniqueSchedulePlanName(`新课表 ${dataStore.schedulePlans.length + 1}`);
};

const ensureCanManageSchedulePlans = async () => {
    await dataStore.syncUnsavedStatus();
    if (dataStore.hasUnsavedChanges) {
        message.warning(schedulePlanBlockedMessage);
        return false;
    }
    return true;
};

const closeSchedulePlanModal = () => {
    if (schedulePlanActionPending.value) return;
    schedulePlanModal.value.show = false;
};

const openSchedulePlanModal = async (mode) => {
    if (!(await ensureCanManageSchedulePlans())) return;
    schedulePlanForm.value = { name: defaultSchedulePlanName(mode) };
    schedulePlanModal.value = { show: true, mode };
};

const finishSchedulePlanNavigation = (successText) => {
    dragStore.cancelDrag();
    dataStore.clearScheduleFocus();
    message.success(successText);
};

const submitSchedulePlanModal = async () => {
    try {
        await schedulePlanFormRef.value?.validate();
    } catch {
        return;
    }

    const name = String(schedulePlanForm.value.name || '').trim();
    if (!name) {
        message.error('请输入课表版本名称');
        return;
    }
    if (!(await ensureCanManageSchedulePlans())) return;

    schedulePlanActionPending.value = true;
    try {
        if (schedulePlanModal.value.mode === 'copy') {
            await dataStore.copySchedulePlan(name);
            finishSchedulePlanNavigation(`已复制并切换到课表版本「${dataStore.activeSchedulePlan?.name || name}」`);
        } else if (schedulePlanModal.value.mode === 'rename') {
            await dataStore.renameSchedulePlan(dataStore.activeSchedulePlanId, name);
            finishSchedulePlanNavigation(`课表版本已重命名为「${dataStore.activeSchedulePlan?.name || name}」`);
        } else {
            await dataStore.createSchedulePlan(name);
            finishSchedulePlanNavigation(`已新建并切换到课表版本「${dataStore.activeSchedulePlan?.name || name}」`);
        }
        schedulePlanModal.value.show = false;
    } catch (error) {
        dialog.error({
            title: '课表版本操作失败',
            content: errorMessage(error),
            positiveText: '确定'
        });
    } finally {
        schedulePlanActionPending.value = false;
    }
};

const handleSchedulePlanSwitch = async (planId) => {
    if (!planId || planId === dataStore.activeSchedulePlanId) return;
    if (!(await ensureCanManageSchedulePlans())) return;

    schedulePlanActionPending.value = true;
    try {
        await dataStore.switchSchedulePlan(planId);
        finishSchedulePlanNavigation(`已切换到课表版本「${dataStore.activeSchedulePlan?.name || '未命名课表'}」`);
    } catch (error) {
        dialog.error({
            title: '切换课表版本失败',
            content: errorMessage(error),
            positiveText: '确定'
        });
    } finally {
        schedulePlanActionPending.value = false;
    }
};

const confirmDeleteSchedulePlan = async () => {
    if (dataStore.schedulePlans.length <= 1) {
        message.warning('不能删除最后一个课表版本。');
        return;
    }
    if (!(await ensureCanManageSchedulePlans())) return;

    const plan = dataStore.activeSchedulePlan;
    if (!plan) return;

    dialog.warning({
        title: '删除课表版本',
        content: `确定删除课表版本「${plan.name}」吗？该版本内的排课记录会一起删除。`,
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: async () => {
            schedulePlanActionPending.value = true;
            try {
                await dataStore.deleteSchedulePlan(plan.id);
                finishSchedulePlanNavigation(`已删除课表版本「${plan.name}」，当前切换到「${dataStore.activeSchedulePlan?.name || '未命名课表'}」`);
            } catch (error) {
                dialog.error({
                    title: '删除课表版本失败',
                    content: errorMessage(error),
                    positiveText: '确定'
                });
            } finally {
                schedulePlanActionPending.value = false;
            }
        }
    });
};

const handleResize = () => {
    const width = document.documentElement.clientWidth;
    collapsed.value = width < 840;
};

onMounted(async () => {
    if (!dataStore.isInitialized) {
        dataStore.initializeData();
    }

    unlistenClose = await appWindow.listen('show-close-dialog', () => {
        triggerConfirm();
    });

    handleResize();
    window.addEventListener('resize', handleResize);
    window.addEventListener('pointerup', handleSchedulePointerEnd);
    window.addEventListener('pointercancel', handleSchedulePointerEnd);

    isMaximized.value = await appWindow.isMaximized();
    unlistenResize = await appWindow.onResized(async () => {
        isMaximized.value = await appWindow.isMaximized();
    });
});

onUnmounted(() => {
    window.removeEventListener('resize', handleResize);
    window.removeEventListener('pointerup', handleSchedulePointerEnd);
    window.removeEventListener('pointercancel', handleSchedulePointerEnd);
    if (unlistenResize) {
        unlistenResize();
    }
    if (unlistenClose) {
        unlistenClose();
    }
});

const renderIcon = (icon) => () => h(NIcon, null, { default: () => h(icon) });

const menuOptions = computed(() => [
    {
        label: '校区总课表',
        key: 'CampusTimetable',
        icon: renderIcon(CalendarIcon),
    },
    {
        label: '教师个人课表',
        key: 'TeacherTimetable',
        icon: renderIcon(PersonIcon),
    },
    {
        label: '问题检查',
        key: 'ScheduleIssues',
        icon: renderIcon(IssueIcon),
    },
    {
        type: 'divider',
        key: 'd1'
    },
    {
        label: '教师信息管理',
        key: 'TeacherManagement',
        icon: renderIcon(PeopleIcon),
    },
    {
        label: '课程信息管理',
        key: 'CourseManagement',
        icon: renderIcon(BookIcon),
    },
    {
        label: '场地/校区管理',
        key: 'VenueManagement',
        icon: renderIcon(HomeIcon),
    },
    {
        type: 'divider',
        key: 'd2'
    },
    {
        label: '自动排课',
        key: 'AutoSchedule',
        icon: renderIcon(SolverIcon),
    },
    {
        label: '保存更改',
        key: 'SaveChanges',
        icon: renderIcon(SaveIcon),
        disabled: !dataStore.hasUnsavedChanges,
    },
    {
        label: '撤销所有更改',
        key: 'RevertChanges',
        icon: renderIcon(UndoIcon),
        disabled: !dataStore.hasUnsavedChanges,
    },
    {
        label: '设置',
        key: 'Settings',
        icon: renderIcon(SettingsIcon),
    }
]);

const activeMenuKey = computed(() => route.name);
const stagingRoutes = new Set(['TeacherTimetable', 'CampusTimetable']);
const isStagingRoute = computed(() => stagingRoutes.has(route.name));
const showStagingTray = computed(() => {
    return isStagingRoute.value && (dragStore.isDragging || dataStore.stagedScheduledClasses.length > 0);
});

const handleSchedulePointerDown = (event) => {
    if (!isStagingRoute.value) return;
    if (event.target.closest?.('.schedule-staging-tray')) return;
    if (event.target.closest?.('.schedule-card__actions')) return;

    const scheduleElement = event.target.closest?.('[data-schedule-id]');
    const scheduleId = scheduleElement?.dataset?.scheduleId;
    if (!scheduleId) return;

    const schedule = dataStore.scheduledClasses.find(item => item.id === scheduleId);
    if (!schedule) return;

    dragStore.startDrag({
        schedule,
        source: { type: route.name === 'TeacherTimetable' ? 'teacher' : 'campus', routeName: route.name },
        event,
    });
};

const handleSchedulePointerEnd = (event) => {
    if (event.target.closest?.('.schedule-staging-tray')) return;
    if (dragStore.isDragging) {
        dragStore.endDrag();
    }
};

const scheduleById = (scheduleId) => dataStore.scheduledClasses.find(schedule => schedule.id === scheduleId);
const isLockedActiveSchedule = (schedule) => Boolean(schedule?.is_locked) && schedule?.is_staged !== true;
const issueOnlyTouchesLockedActiveSchedules = (issue) => {
    const scheduleIds = Array.isArray(issue.schedule_ids) ? issue.schedule_ids : [];
    return scheduleIds.length > 0 && scheduleIds.every(scheduleId => isLockedActiveSchedule(scheduleById(scheduleId)));
};

const renderIssueList = (intro, issues) => h('div', { class: 'solver-precheck' }, [
    h('p', intro),
    h('ul', { class: 'solver-precheck__list' }, issues.slice(0, 6).map(issue => h('li', { key: issue.id }, issue.message))),
    issues.length > 6 ? h('p', { class: 'solver-precheck__more' }, `还有 ${issues.length - 6} 条，请到问题检查页处理。`) : null,
].filter(Boolean));

const runSolver = async () => {
    dataStore.isSolving = true;
    try {
        message.info('正在排课...');
        await dataStore.flushPendingChanges();
        const solvedData = await invoke('run_solver');
        dataStore.replaceAllData(solvedData);
        message.success('自动排课完成！新的课表已生成，请检查并保存。');
    } catch (err) {
        console.error('Solver failed:', err);
        dialog.error({
            title: '排课失败',
            content: err?.message || String(err),
            positiveText: '好的'
        });
    } finally {
        dataStore.isSolving = false;
    }
};

const runSolverWithPrecheck = () => {
    const lockedHardIssues = dataStore.scheduleIssues.filter(issue => {
        return issue.severity === 'error' && issueOnlyTouchesLockedActiveSchedules(issue);
    });

    if (lockedHardIssues.length > 0) {
        dialog.warning({
            title: '自动排课前需要处理硬冲突',
            content: () => renderIssueList('以下锁定课程存在硬冲突。请先解锁或调整后再自动排课。', lockedHardIssues),
            positiveText: '返回问题检查',
            negativeText: '稍后处理',
            onPositiveClick: () => router.push({ name: 'ScheduleIssues' })
        });
        return;
    }

    const stagedCount = dataStore.stagedScheduledClasses.length;
    if (stagedCount > 0) {
        dialog.warning({
            title: '暂存区课程不参与自动排课',
            content: `当前暂存区有 ${stagedCount} 张课程卡片，它们不会传给自动排课，也不会在排课后丢失。`,
            positiveText: '继续自动排课',
            negativeText: '取消',
            onPositiveClick: runSolver
        });
        return;
    }

    runSolver();
};

const handleMenuSelect = async (key) => {
    if (key === 'SaveChanges') {
        try {
            await dataStore.commitChanges();
            message.success('数据已成功保存！');
            await dataStore.syncUnsavedStatus();
        } catch (err) {
            console.error('Save failed:', err);
            message.error(`保存失败: ${err}`);
        }
        return;
    }
    if (key === 'RevertChanges') {
        dialog.warning({
            title: '确认撤销更改',
            content: '确定要撤销所有未保存的更改吗？此操作将还原到上次保存的状态。',
            positiveText: '确认',
            negativeText: '取消',
            onPositiveClick: async () => {
                try {
                    await dataStore.revertChanges();
                    message.success('所有更改已成功撤销。');
                    await dataStore.syncUnsavedStatus();
                } catch (err) {
                    console.error('Revert failed:', err);
                    message.error(`撤销失败: ${err}`);
                }
            }
        });
        return;
    }
    if (key === 'AutoSchedule') {
        runSolverWithPrecheck();
        return;
    }
    router.push({ name: key });
};

const minimizeWindow = async () => {
    await appWindow.minimize();
};
const toggleMaximizeWindow = async () => {
    await appWindow.toggleMaximize();
};

const cleanup = () => {
    isDialogVisible.value = false;
};

const performFinalAction = async (shouldSave) => {
    try {
        await invoke('finalize_and_close', { save: shouldSave });
    } finally {
        cleanup();
    }
};

const triggerConfirm = async () => {
    if (isDialogVisible.value) return;

    await dataStore.syncUnsavedStatus();
    isDialogVisible.value = true;

    if (dataStore.hasUnsavedChanges) {
        dialog.warning({
            title: '有未保存的更改',
            content: '您当前有修改尚未保存。退出应用前，是否需要保存？',
            positiveText: '保存',
            negativeText: '不保存',
            closable: true,
            maskClosable: true,
            onPositiveClick: () => performFinalAction(true),
            onNegativeClick: () => performFinalAction(false),
            onClose: cleanup,
            onEsc: cleanup,
            onMaskClick: cleanup
        });
    } else {
        dialog.warning({
            title: '确认退出应用',
            content: '确定要退出应用吗？',
            positiveText: '确定',
            negativeText: '取消',
            closable: true,
            maskClosable: true,
            onPositiveClick: () => performFinalAction(false),
            onNegativeClick: cleanup,
            onClose: cleanup,
            onEsc: cleanup,
            onMaskClick: cleanup
        });
    }
};
</script>

<style scoped>
h1 {
    font-size: 1.5em;
    margin: 0;
    color: #333;
}

.app-spin,
.app-shell {
    height: 100vh;
    overflow: hidden;
}

:deep(.app-spin > .n-spin-container),
:deep(.app-spin > .n-spin-container > .n-spin-content) {
    height: 100%;
    min-height: 0;
}

.app-main {
    height: calc(100vh - 48px);
    min-height: 0;
    overflow: hidden;
}

.app-titlebar {
    height: 48px;
    padding: 0 12px 0 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    user-select: none;
    --wails-draggable: drag;
}

.app-titlebar__right {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    --wails-draggable: no-drag;
}

.schedule-plan-switcher {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
}

.schedule-plan-switcher__label {
    color: #606266;
    font-size: 13px;
    white-space: nowrap;
}

.schedule-plan-switcher__select {
    width: 220px;
}

.schedule-plan-modal__actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
}

@media (max-width: 980px) {
    .schedule-plan-switcher__label {
        display: none;
    }

    .schedule-plan-switcher__select {
        width: 168px;
    }
}

@media (max-width: 760px) {
    .schedule-plan-switcher {
        display: none;
    }
}

.main-layout__content {
    height: 100%;
    min-height: 0;
    overflow: hidden;
}
.main-layout__content :deep(.n-layout-scroll-container) {
    height: 100%;
    min-height: 0;
    overflow: hidden !important;
}


.main-layout__route {
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
}

.main-layout__loading {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
}

.n-layout-header {
    background-color: #f8f8f8;
}

 .solver-precheck__list {
    margin: 8px 0 0;
    padding-left: 20px;
}

.solver-precheck__more {
    margin: 8px 0 0;
    color: #606266;
}
</style>
