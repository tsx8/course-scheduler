<template>
    <n-spin :show="dataStore.isSolving" size="large">
        <template #description>
            正在排课中……
        </template>
        <n-layout style="height: 100vh">
            <n-layout-header data-tauri-drag-region="true"
                style="height: 48px; padding: 0 12px 0 24px; display: flex; align-items: center; justify-content: space-between; user-select: none;"
                bordered>
                <h1 data-tauri-drag-region="true" style="font-size: 1.2em; margin: 0; color: #333; user-select: none;">
                    排课管理系统
                </h1>
                <div data-tauri-drag-region="false" style="display: flex; align-items: center; gap: 12px;">
                    <n-space align="center" :size="8" v-if="!isLoginPage && authStore.currentUser">
                        <n-text depth="3" style="font-size: 14px;">
                            {{ authStore.currentUser.username }} ({{ authStore.currentUser.role ||
                                (authStore.isScheduler ?
                                    '排课员' : '教师') }})
                        </n-text>
                        <n-button size="small" quaternary @click="handleLogout" title="退出登录">
                            <template #icon><n-icon :component="LogoutIcon" /></template>
                            退出
                        </n-button>
                    </n-space>

                    <!-- Window controls -->
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
            <n-layout has-sider>
                <n-layout-sider v-if="!isLoginPage" bordered collapse-mode="width" show-trigger v-model:collapsed="collapsed">
                    <n-menu :options="menuOptions" :value="activeMenuKey" @update:value="handleMenuSelect" />
                </n-layout-sider>
                <n-layout-content
                    :content-style="isLoginPage ? 'padding: 0; display: flex; flex-direction: column;' : 'padding: 24px; display: flex; flex-direction: column;'"
                    :native-scrollbar="false">
                    <router-view v-if="dataStore.isInitialized || isLoginPage" />
                    <div v-else style="display: flex; justify-content: center; align-items: center; height: 100%;">
                        <n-spin size="large" />
                        <p style="margin-left: 12px;">正在加载数据...</p>
                    </div>
                </n-layout-content>
            </n-layout>
        </n-layout>
    </n-spin>
</template>

<script setup>
import { h, ref, computed, onMounted, onUnmounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useDataStore } from '../stores/data';
import { useAuthStore } from '../stores/auth';
import {
    NLayout, NLayoutHeader, NLayoutSider, NLayoutContent, NMenu, NIcon,
    NSpin, NButton, NButtonGroup, NSpace, NText, useDialog, useMessage, useNotification
} from 'naive-ui';
import {
    BookOutline as BookIcon,
    PeopleOutline as PeopleIcon,
    HomeOutline as HomeIcon,
    CalendarOutline as CalendarIcon,
    PersonOutline as PersonIcon,
    RemoveOutline as MinimizeIcon,
    SquareOutline as MaximizeIcon,
    ResizeOutline as RestoreIcon,
    CloseSharp as CloseIcon,
    SettingsOutline as SettingsIcon,
    RocketOutline as SolverIcon,
    SaveOutline as SaveIcon,
    RefreshOutline as UndoIcon,
    LogOutOutline as LogoutIcon,
    PeopleCircleOutline as UsersIcon,
    ListOutline as ListIcon
} from '@vicons/ionicons5';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';

const dataStore = useDataStore();
const authStore = useAuthStore();
const route = useRoute();
const router = useRouter();
const dialog = useDialog();
const message = useMessage();

const appWindow = getCurrentWebviewWindow();
const isMaximized = ref(false);
const isDialogVisible = ref(false);
const isLoginPage = computed(() => route.name === 'Login');
let unlistenResize = null;
let unlistenClose = null;

const collapsed = ref(false);

const handleResize = () => {
    const width = document.documentElement.clientWidth;
    if (width < 840) {
        collapsed.value = true;
    } else {
        collapsed.value = false;
    }
};

onMounted(async () => {
    if (!dataStore.isInitialized) {
        dataStore.initializeData();
    }

    unlistenClose = await appWindow.listen('show-close-dialog', () => {
        triggerConfirm('exit');
    });

    handleResize();
    window.addEventListener('resize', handleResize);

    isMaximized.value = await appWindow.isMaximized();
    unlistenResize = await appWindow.onResized(async () => {
        isMaximized.value = await appWindow.isMaximized();
    });
});

onUnmounted(() => {
    window.removeEventListener('resize', handleResize);
    if (unlistenResize) {
        unlistenResize();
    }
    if (unlistenClose) {
        unlistenClose();
    }
});

const renderIcon = (icon) => () => h(NIcon, null, { default: () => h(icon) });

const menuOptions = computed(() => {
    const options = [
        {
            label: '校区总课表',
            key: 'CampusTimetable',
            icon: renderIcon(CalendarIcon),
            show: authStore.isScheduler, // Only show to Schedulers
        },
        {
            label: '教师个人课表',
            key: 'TeacherTimetable',
            icon: renderIcon(PersonIcon),
        },
    ];

    // Add divider and management options only for Schedulers
    if (authStore.isScheduler) {
        options.push(
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
                label: '用户管理',
                key: 'UserManagement',
                icon: renderIcon(UsersIcon),
            },
            {
                label: '审计日志',
                key: 'AuditLogs',
                icon: renderIcon(ListIcon),
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
            }
        );
    }

    options.push(
        {
            label: '设置',
            key: 'Settings',
            icon: renderIcon(SettingsIcon),
        }
    );

    return options.filter(opt => opt.show !== false);
});

const activeMenuKey = computed(() => route.name);

const handleMenuSelect = async (key) => {
    if (key === 'SaveChanges') {
        try {
            await dataStore.commitChanges();
            message.success('数据已成功保存！');
            await dataStore.syncUnsavedStatus();
        } catch (err) {
            console.error("Save failed:", err);
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
                    console.error("Revert failed:", err);
                    message.error(`撤销失败: ${err}`);
                }
            }
        });
        return;
    }
    if (key === 'AutoSchedule') {
        dataStore.isSolving = true;
        try {
            message.info('正在排课...');
            const solvedData = await invoke('run_solver', {
                sessionId: authStore.sessionId
            });
            dataStore.replaceAllData(solvedData);
            message.success('自动排课完成！新的课表已生成，请检查并保存。');
        } catch (err) {
            console.error("Solver failed:", err);
            dialog.error({
                title: '排课失败',
                content: err,
                positiveText: '好的'
            });
        } finally {
            dataStore.isSolving = false;
        }
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

const handleLogout = async () => {
    triggerConfirm('logout');
};

const performFinalAction = async (actionType, shouldSave) => {
    const sessionId = authStore.sessionId;

    if (actionType === 'exit') {
        await invoke('finalize_and_close', { sessionId, save: shouldSave });
    } else if (actionType === 'logout') {
        if (shouldSave) {
            await dataStore.commitChanges();
        } else if (dataStore.hasUnsavedChanges) {
            await dataStore.revertChanges();
        }

        try {
            await authStore.logout();
            message.success('已退出登录');
            router.push('/login');
        } catch (error) {
            console.error('Logout error:', error);
            message.error('退出登录失败');
        } finally {
            cleanup();
        }
    }
    isDialogVisible.value = false;
};

const triggerConfirm = async (actionType) => {
    if (isDialogVisible.value) return;

    await dataStore.syncUnsavedStatus();

    isDialogVisible.value = true;

    const isExit = actionType === 'exit';
    const actionName = isExit ? '退出应用' : '退出登录';

    if (dataStore.hasUnsavedChanges) {
        dialog.warning({
            title: `有未保存的更改`,
            content: `您当前有修改尚未保存。在${actionName}前，是否需要保存？`,
            positiveText: '保存并退出',
            negativeText: '不保存直接退出',
            closable: true,
            maskClosable: true,
            onPositiveClick: () => performFinalAction(actionType, true),
            onNegativeClick: () => performFinalAction(actionType, false),
            onClose: cleanup,
            onEsc: cleanup,
            onMaskClick: cleanup
        });
    } else {
        dialog.warning({
            title: `确认${actionName}`,
            content: `确定要${actionName}吗？`,
            positiveText: '确定',
            negativeText: '取消',
            closable: true,
            maskClosable: true,
            onPositiveClick: () => performFinalAction(actionType, false),
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

.n-layout-header {
    background-color: #f8f8f8;
}
</style>