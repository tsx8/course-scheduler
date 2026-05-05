<template>
    <n-spin class="app-spin" :show="dataStore.isSolving" size="large">
        <template #description>
            正在排课中……
        </template>
        <n-layout class="app-shell">
            <n-layout-header
                style="height: 48px; padding: 0 12px 0 24px; display: flex; align-items: center; justify-content: space-between; user-select: none; --wails-draggable: drag;"
                bordered>
                <h1 style="font-size: 1.2em; margin: 0; color: #333; user-select: none; --wails-draggable: drag;">
                    排课管理系统
                </h1>
                <div style="display: flex; align-items: center; gap: 12px; --wails-draggable: no-drag;">
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
    NSpin, NButton, NButtonGroup, useDialog, useMessage
} from 'naive-ui';
import {
    BookOutline as BookIcon,
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
            positiveText: '保存并退出',
            negativeText: '不保存直接退出',
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
