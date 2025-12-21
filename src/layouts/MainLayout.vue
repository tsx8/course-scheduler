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
                <div data-tauri-drag-region="false">
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
                <n-layout-sider bordered collapse-mode="width" show-trigger>
                    <n-menu :options="menuOptions" :value="activeMenuKey" @update:value="handleMenuSelect" />
                </n-layout-sider>
                <n-layout-content content-style="padding: 24px; display: flex; flex-direction: column;"
                    :native-scrollbar="false">
                    <router-view v-if="dataStore.isInitialized" />
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
import {
    NLayout, NLayoutHeader, NLayoutSider, NLayoutContent, NMenu, NIcon,
    NSpin, NButton, NButtonGroup, useDialog, useMessage, useNotification
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
    RefreshOutline as UndoIcon
} from '@vicons/ionicons5';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const dataStore = useDataStore();
const route = useRoute();
const router = useRouter();
const dialog = useDialog();
const message = useMessage();
const notification = useNotification();

const appWindow = getCurrentWebviewWindow();
const isMaximized = ref(false);
const isDialogVisible = ref(false);
let unlistenResize = null;

onMounted(async () => {
    if (!dataStore.isInitialized) {
        dataStore.initializeData();
    }
    
    // Existing close dialog listener
    await appWindow.listen('show-close-dialog', () => {
        showConfirmDialog();
    });

    isMaximized.value = await appWindow.isMaximized();
    unlistenResize = await appWindow.onResized(async () => {
        isMaximized.value = await appWindow.isMaximized();
    });
});

onUnmounted(() => {
    if (unlistenResize) {
        unlistenResize();
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
    },
]);

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
            const solvedData = await invoke('run_solver');
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

const showConfirmDialog = () => {
    if (isDialogVisible.value) return;

    isDialogVisible.value = true;

    dialog.warning({
        title: '退出应用',
        content: '你希望在退出前保存当前所做的修改吗？',
        positiveText: '保存',
        negativeText: '不保存',
        onPositiveClick: () => {
            invoke('finalize_and_close', { save: true }).catch(err => {
                console.error("Failed to finalize and close:", err);
            });
        },
        onNegativeClick: () => {
            invoke('finalize_and_close', { save: false }).catch(err => {
                console.error("Failed to finalize and close:", err);
            });
        },
        onClose: () => {
            isDialogVisible.value = false;
        },
        onEsc: () => {
            isDialogVisible.value = false;
        },
        onMaskClick: () => {
            isDialogVisible.value = false;
        },
    });
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