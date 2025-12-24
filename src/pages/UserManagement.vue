<script setup>
import { ref, computed, onMounted, onUnmounted, h, reactive, watch } from 'vue';
import { useAuthStore } from '../stores/auth';
import { useDataStore } from '../stores/data';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
    NSpace, NFlex, NH2, NButton, NDataTable, NIcon, NModal, NForm, NFormItem,
    NInput, NSelect, NTag, useMessage, useDialog
} from 'naive-ui';
import { AddOutline as AddIcon, CreateOutline as EditIcon, KeyOutline as KeyIcon, TrashOutline as DeleteIcon } from '@vicons/ionicons5';

const committedTeachers = ref([]);
const loadCommittedTeachers = async () => {
    try {
        committedTeachers.value = await invoke('list_committed_teachers');
    } catch (error) {
        console.error('Failed to load committed teachers:', error);
    }
};
const authStore = useAuthStore();
const dataStore = useDataStore();
const message = useMessage();
const dialog = useDialog();

const listeners = [];

const pagination = reactive({
    page: 1,
    pageSize: 10,
    showSizePicker: true,
    pageSizes: [10, 20, 50],
    onChange: (page) => {
        pagination.page = page;
    },
    onUpdatePageSize: (pageSize) => {
        pagination.pageSize = pageSize;
        pagination.page = 1;
    },
    prefix: () => `共 ${users.value.length} 个用户`
});

const users = ref([]);
const loading = ref(false);

const showCreateModal = ref(false);
const createForm = ref({
    username: '',
    password: '',
    role_id: null,
    teacher_id: null
});

const showEditModal = ref(false);
const editForm = ref({
    user_id: '',
    username: '',
    role_id: null,
    teacher_id: null
});

const showResetPasswordModal = ref(false);
const resetPasswordForm = ref({
    user_id: '',
    username: '',
    new_password: ''
});

const roleOptions = computed(() => {
    return dataStore.roles.map(role => ({
        label: role.name,
        value: role.id
    }));
});

const teacherOptions = computed(() => {
    return [
        { label: '(无)', value: null },
        ...committedTeachers.value.map(t => ({
            label: t.name,
            value: t.id
        }))
    ];
});

const loadUsers = async () => {
    loading.value = true;
    try {
        const result = await invoke('list_users');
        users.value = result;
    } catch (error) {
        console.error('Failed to load users:', error);
        message.error(`加载用户列表失败: ${error}`);
    } finally {
        loading.value = false;
    }
};

const handleCreateUser = async () => {
    if (!createForm.value.username || !createForm.value.password || !createForm.value.role_id) {
        message.warning('请填写所有必填字段');
        return;
    }

    try {
        await invoke('create_user', {
            username: createForm.value.username,
            password: createForm.value.password,
            roleId: createForm.value.role_id,
            teacherId: createForm.value.teacher_id,
            sessionId: authStore.sessionId
        });

        message.success('用户创建成功');
        showCreateModal.value = false;

        createForm.value = {
            username: '',
            password: '',
            role_id: null,
            teacher_id: null
        };

        await loadUsers();
    } catch (error) {
        console.error('Failed to create user:', error);
        message.error(`创建用户失败: ${error}`);
    }
};

const openEditModal = (user) => {
    editForm.value = {
        user_id: user.id,
        username: user.username,
        role_id: user.role_id,
        teacher_id: user.teacher_id
    };
    showEditModal.value = true;
};

const handleUpdateUser = async () => {
    if (!editForm.value.role_id) {
        message.warning('角色不能为空');
        return;
    }

    try {
        await invoke('update_user', {
            userId: editForm.value.user_id,
            roleId: editForm.value.role_id,
            teacherId: editForm.value.teacher_id,
            sessionId: authStore.sessionId
        });

        message.success('角色更新成功');
        showEditModal.value = false;
        await loadUsers();
    } catch (error) {
        console.error('Failed to update user role:', error);
        message.error(`更新角色失败: ${error}`);
    }
};

const openResetPasswordModal = (user) => {
    resetPasswordForm.value = {
        user_id: user.id,
        username: user.username,
        new_password: ''
    };
    showResetPasswordModal.value = true;
};

const handleResetPassword = async () => {
    if (!resetPasswordForm.value.new_password) {
        message.warning('请输入新密码');
        return;
    }

    try {
        await invoke('reset_password', {
            userId: resetPasswordForm.value.user_id,
            newPassword: resetPasswordForm.value.new_password,
            sessionId: authStore.sessionId
        });

        message.success('密码重置成功');
        showResetPasswordModal.value = false;

        resetPasswordForm.value = {
            user_id: '',
            username: '',
            new_password: ''
        };
    } catch (error) {
        console.error('Failed to reset password:', error);
        message.error(`重置密码失败: ${error}`);
    }
};

const confirmDeleteUser = (user) => {
    dialog.warning({
        title: '确认删除用户',
        content: `确定要删除用户【${user.username}】吗？该操作不可撤销。`,
        positiveText: '确认删除',
        negativeText: '取消',
        onPositiveClick: () => {
            handleDeleteUser(user.id);
        },
    });
};

const handleDeleteUser = async (userId) => {
    try {
        await invoke('delete_user', {
            userId,
            sessionId: authStore.sessionId
        });
        message.success('用户已删除');
        await loadUsers();
    } catch (error) {
        console.error('Failed to delete user:', error);
        message.error(`删除用户失败: ${error}`);
    }
};

const columns = computed(() => [
    {
        title: '用户名',
        key: 'username',
        width: '10%',
        ellipsis: { tooltip: true }
    },
    {
        title: '角色',
        key: 'role',
        width: '10%',
        render(row) {
            return h(NTag, {
                type: row.role === 'Scheduler' ? 'info' : 'success',
                size: 'small'
            }, { default: () => row.role === 'Scheduler' ? '排课员' : '教师' });
        }
    },
    {
        title: '关联教师',
        key: 'teacher_name',
        width: '10%',
        render(row) {
            return row.teacher_name || '-';
        }
    },
    {
        title: '创建时间',
        key: 'created_at',
        // width: '20%',
        render(row) {
            return row.created_at ? new Date(row.created_at).toLocaleString('zh-CN') : '-';
        }
    },
    {
        title: '最后登录',
        key: 'last_login',
        // width: '20%',
        render(row) {
            return row.last_login ? new Date(row.last_login).toLocaleString('zh-CN') : '从未登录';
        }
    },
    {
        title: '操作',
        key: 'actions',
        width: '20%',
        // fixed: 'right',
        render(row) {
            const isCurrentUser = row.id === authStore.currentUser?.id;

            return h(NSpace, { size: 8 }, () => [
                h(NButton, {
                    size: 'small',
                    type: 'warning',
                    onClick: () => openResetPasswordModal(row)
                }, {
                    icon: () => h(NIcon, { component: KeyIcon }),
                }),

                h(NButton, {
                    size: 'small',
                    type: 'info',
                    onClick: () => openEditModal(row),
                    disabled: isCurrentUser
                }, {
                    icon: () => h(NIcon, { component: EditIcon }),
                }),

                h(NButton, {
                    size: 'small',
                    type: 'error',
                    disabled: isCurrentUser,
                    onClick: () => confirmDeleteUser(row)
                }, {
                    icon: () => h(NIcon, { component: DeleteIcon })
                })
            ]);
        }
    }
]);

onMounted(async () => {
    const refreshData = () => {
        loadUsers();
        loadCommittedTeachers();
    };

    refreshData();

    listeners.push(await listen('commit-completed', refreshData));
    listeners.push(await listen('data-reloaded', refreshData));
});

onUnmounted(() => {
    listeners.forEach(unlisten => unlisten());
    listeners.length = 0;
});

watch(showCreateModal, (val) => {
    if (val) loadCommittedTeachers();
});
</script>

<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-flex justify="space-between" align="center">
                <n-h2 style="margin: 0;">用户管理</n-h2>
                <n-button type="primary" @click="showCreateModal = true">
                    <template #icon><n-icon :component="AddIcon" /></template>
                    创建用户
                </n-button>
            </n-flex>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-data-table :columns="columns" :data="users" :loading="loading" :pagination="pagination" :bordered="true"
                :single-line="false" style="width: 100%;" />

        </n-layout-content>

        <n-modal v-model:show="showCreateModal" preset="dialog" title="创建用户" positive-text="创建" negative-text="取消"
            @positive-click="handleCreateUser">
            <n-form :model="createForm" label-placement="left" label-width="100">
                <n-form-item label="用户名" required>
                    <n-input v-model:value="createForm.username" placeholder="请输入用户名" />
                </n-form-item>
                <n-form-item label="密码" required>
                    <n-input v-model:value="createForm.password" type="password" placeholder="请输入密码"
                        show-password-on="click" />
                </n-form-item>
                <n-form-item label="角色" required>
                    <n-select v-model:value="createForm.role_id" :options="roleOptions" placeholder="请选择角色" />
                </n-form-item>
                <n-form-item label="关联教师">
                    <n-select v-model:value="createForm.teacher_id" :options="teacherOptions" placeholder="选择关联教师(可选)"
                        clearable />
                </n-form-item>
            </n-form>
        </n-modal>

        <n-modal v-model:show="showEditModal" preset="dialog" title="编辑用户信息" positive-text="保存更改" negative-text="取消"
            @positive-click="handleUpdateUser">
            <n-form :model="editForm" label-placement="left" label-width="100" style="margin-top: 16px;">
                <n-form-item label="用户名">
                    <n-input :value="editForm.username" disabled placeholder="用户名不可修改" />
                </n-form-item>
                <n-form-item label="用户角色">
                    <n-select v-model:value="editForm.role_id" :options="roleOptions" placeholder="请选择角色" />
                </n-form-item>
                <n-form-item label="关联教师">
                    <n-select v-model:value="editForm.teacher_id" :options="teacherOptions" placeholder="选择关联教师(可选)"
                        clearable />
                </n-form-item>
            </n-form>
        </n-modal>

        <n-modal v-model:show="showResetPasswordModal" preset="dialog" title="重置密码" positive-text="重置"
            negative-text="取消" @positive-click="handleResetPassword">
            <n-form :model="resetPasswordForm" label-placement="left" label-width="100">
                <n-form-item label="用户名">
                    <n-input :value="resetPasswordForm.username" disabled />
                </n-form-item>
                <n-form-item label="新密码" required>
                    <n-input v-model:value="resetPasswordForm.new_password" type="password" placeholder="请输入新密码"
                        show-password-on="click" />
                </n-form-item>
            </n-form>
        </n-modal>
    </n-layout>
</template>
