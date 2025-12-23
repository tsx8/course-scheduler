<template>
    <n-layout style="height: calc(100vh - 96px);">
        <n-layout-header bordered style="padding: 12px 24px;">
            <n-h2 style="margin: 0;">审计日志</n-h2>
        </n-layout-header>

        <n-layout-content content-style="padding: 24px;" style="height: calc(100vh - 156px); overflow: auto;"
            :native-scrollbar="false">
            <n-card title="筛选" style="margin-bottom: 16px;" size="small">
                <n-flex align="center" :size="12">
                    <n-form-item label="用户名" label-placement="left" :show-feedback="false">
                        <n-input v-model:value="filterUsername" placeholder="搜索用户名" clearable style="width: 120px;" />
                    </n-form-item>
                    <n-form-item label="操作" label-placement="left" :show-feedback="false">
                        <n-select v-model:value="filterActionType" :options="actionTypeOptions" placeholder="全部操作"
                            clearable style="width: 120px;" />
                    </n-form-item>
                    <n-form-item label="日期" label-placement="left" :show-feedback="false">
                        <n-date-picker v-model:value="filterDateRange" type="daterange" clearable />
                    </n-form-item>
                    <n-button type="primary" @click="loadAuditLogs">
                        查询
                    </n-button>
                </n-flex>
            </n-card>

            <n-data-table :columns="columns" :data="auditLogs" :pagination="paginationConfig" :loading="loading"
                :row-key="row => row.id" :remote="true" :bordered="true" />
        </n-layout-content>

        <n-modal v-model:show="showDetailModal" preset="card" title="提交详情" style="width: 600px; max-width: 90vw;">
            <n-scrollbar style="max-height: 60vh;">
                <pre class="json-viewer">{{ currentDetailJson }}</pre>
            </n-scrollbar>
        </n-modal>
    </n-layout>
</template>

<script setup>
import { ref, computed, h, onMounted, onUnmounted } from 'vue';
import {
    NLayout, NLayoutHeader, NLayoutContent, NCard, NFlex, NFormItem, NInput, NSelect,
    NDatePicker, NButton, NDataTable, NText, NH2, NTag, NModal, NScrollbar, useMessage, NIcon
} from 'naive-ui';
import { DocumentTextOutline, ArrowForwardOutline } from '@vicons/ionicons5';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '../stores/auth';

const authStore = useAuthStore();
const message = useMessage();

const auditLogs = ref([]);
const loading = ref(false);
const currentPage = ref(1);
const perPage = ref(10);
const totalEntries = ref(0);
const totalPages = ref(0);

const listeners = [];

const filterUsername = ref('');
const filterActionType = ref(null);
const filterDateRange = ref(null);

const showDetailModal = ref(false);
const currentDetailJson = ref('');

// Action type options (matching backend enum)
const actionTypeOptions = [
    { label: '用户登录', value: 'LOGIN' },
    { label: '用户登出', value: 'LOGOUT' },
    { label: '登录失败', value: 'LOGIN_FAILED' },
    { label: '创建用户', value: 'USER_CREATED' },
    { label: '自动创建用户', value: 'AUTO_USER_CREATED' },
    { label: '用户更新', value: 'USER_UPDATED' },
    { label: '密码重置', value: 'PASSWORD_RESET' },
    { label: '修改密码', value: 'PASSWORD_CHANGED' },
    { label: '删除用户', value: 'USER_DELETED' },
    { label: '修改排课', value: 'SCHEDULE_MODIFIED' },
    { label: '运行求解器', value: 'SOLVER_RUN' },
    { label: '提交数据', value: 'COMMIT_OPERATION' },
    { label: '回滚数据', value: 'REVERT_OPERATION' },
    { label: '导出数据', value: 'DATA_EXPORTED' },
    { label: '导入数据', value: 'DATA_IMPORTED' },
    { label: '创建教师', value: 'TEACHER_CREATED' },
    { label: '更新教师', value: 'TEACHER_UPDATED' },
    { label: '删除教师', value: 'TEACHER_DELETED' },
    { label: '创建课程', value: 'COURSE_CREATED' },
    { label: '更新课程', value: 'COURSE_UPDATED' },
    { label: '删除课程', value: 'COURSE_DELETED' },
    { label: '新增场地', value: 'VENUE_CREATED' },
    { label: '更新场地', value: 'VENUE_UPDATED' },
    { label: '删除场地', value: 'VENUE_DELETED' },
    { label: '新增校区', value: 'CAMPUS_CREATED' },
    { label: '更新校区', value: 'CAMPUS_UPDATED' },
    { label: '删除校区', value: 'CAMPUS_DELETED' },
    { label: '时间段变更', value: 'TIME_SLOT_MODIFIED' },
    { label: '工作日变更', value: 'DAY_MODIFIED' },
];

// Table columns
const columns = [
    {
        title: '时间',
        key: 'timestamp',
        width: 180,
        render: (row) => {
            const date = new Date(row.timestamp);
            return date.toLocaleString('zh-CN', {
                year: 'numeric',
                month: '2-digit',
                day: '2-digit',
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                hour12: false
            });
        }
    },
    {
        title: '用户',
        key: 'username',
        width: 120
    },
    {
        title: '操作',
        key: 'action_type',
        width: 150,
        render: (row) => {
            const option = actionTypeOptions.find(opt => opt.value === row.action_type);
            const label = option ? option.label : row.action_type;

            // Color mapping for different action types
            let type = 'default';
            if (row.action_type.includes('LOGIN')) type = 'success';
            else if (row.action_type.includes('DELETE')) type = 'error';
            else if (row.action_type.includes('CREATE') || row.action_type.includes('ADD')) type = 'info';
            else if (row.action_type.includes('UPDATE') || row.action_type.includes('CHANGED') || row.action_type.includes('MODIFIED')) type = 'warning';
            else if (row.action_type === 'COMMIT_OPERATION') type = 'primary';

            return h(NTag, { type }, { default: () => label });
        }
    },
    {
        title: '详细信息',
        key: 'change_details',
        render: (row) => {
            if (row.action_type === 'COMMIT_OPERATION') {
                return h(NButton, {
                    size: 'small',
                    secondary: true,
                    type: 'info',
                    onClick: () => openDetailModal(row.change_details)
                }, {
                    icon: () => h(NIcon, { component: DocumentTextOutline }),
                    default: () => '查看提交详情'
                });
            }

            if (!row.change_details) return h(NText, { depth: 3 }, { default: () => '-' });

            try {
                const raw = JSON.parse(row.change_details);
                const list = [];

                let targetName = raw.target_name || raw.username || raw.file_path || '未知目标';
                if (typeof targetName === 'object' && targetName !== null) {
                    targetName = JSON.stringify(targetName);
                }

                list.push(h(NFlex, { align: 'center', size: 6, style: 'margin-bottom: 6px;' }, {
                    default: () => [
                        h(NTag, { size: 'small', type: 'primary', bordered: false }, { default: () => '目标' }),
                        h(NText, { strong: true }, { default: () => targetName })
                    ]
                }));

                const contentNodes = [];

                if (raw.changes && Array.isArray(raw.changes)) {
                    raw.changes.forEach(change => {
                        const oldVal = (typeof change.old === 'object' && change.old !== null) ? JSON.stringify(change.old) : String(change.old ?? '');
                        const newVal = (typeof change.new === 'object' && change.new !== null) ? JSON.stringify(change.new) : String(change.new ?? '');
                        contentNodes.push(h('div', { style: 'display: flex; align-items: center; gap: 4px;' }, [
                            h(NText, { depth: 3 }, { default: () => `${change.label || translateField(change.field)}: ` }),
                            h(NText, { delete: true, depth: 3 }, { default: () => oldVal.length === 0 ? '无' : oldVal }),
                            h(NIcon, { size: 12 }, { default: () => h(ArrowForwardOutline) }),
                            h(NText, { type: 'success' }, { default: () => newVal.length === 0 ? '无' : newVal })
                        ]));
                    });
                }

                const ignoreKeys = ['target_name', 'changes', 'id', 'user_id', 'diff'];
                Object.keys(raw).forEach(key => {
                    if (ignoreKeys.includes(key)) return;

                    const label = translateField(key);
                    const value = raw[key];

                    let displayValue = String(value);
                    if (typeof value === 'object' && value !== null) {
                        displayValue = JSON.stringify(value);
                    }

                    if (key === 'action') {
                        contentNodes.unshift(h('div', [
                            h(NText, { strong: true }, { default: () => displayValue })
                        ]));
                    } else {
                        contentNodes.push(h('div', [
                            h(NText, { depth: 3 }, { default: () => `${label}: ` }),
                            h(NText, null, { default: () => displayValue })
                        ]));
                    }
                });

                if (contentNodes.length > 0) {
                    list.push(h(NFlex, { align: 'start', size: 6 }, {
                        default: () => [
                            h(NTag, { size: 'small', type: 'default', bordered: false }, { default: () => '内容' }),
                            h('div', { style: 'font-size: 13px; line-height: 1.6;' }, contentNodes)
                        ]
                    }));
                }

                return h('div', { style: 'padding: 4px 0;' }, list);
            } catch (e) {
                console.warn('JSON parse error', e);
                return h(NText, { depth: 3 }, { default: () => row.change_details });
            }
        }
    }
];

const openDetailModal = (details) => {
    try {
        const obj = typeof details === 'string' ? JSON.parse(details) : details;
        currentDetailJson.value = JSON.stringify(obj, null, 2);
        showDetailModal.value = true;
    } catch (e) {
        currentDetailJson.value = String(details);
        showDetailModal.value = true;
    }
};

const translateField = (field) => {
    const dict = {
        'role': '权限角色',
        'role_id': '权限角色',
        'role_name': '角色',
        'teacher_id': '关联教师ID',
        'teacher_name': '关联教师',
        'name': '名称',
        'value': '内容',
        'max_teaching_hours': '最大学时',
        'is_only_shahe': '仅沙河校区',
        'venue_id': '场地',
        'campus_id': '校区',
        'capacity': '容量',
        'corresponding_hours': '对应学时',
        'teaches_count': '关联课程数',
        'venue_count': '关联场地数',
        'username': '用户名',
        'action': '动作',
        'reason': '原因',
        'description': '描述',
        'course_id': '课程',
        'course_name': '课程',
        'day_id': '日期',
        'time_id': '时段',
        'old_data': '原数据',
        'target_name': '目标',
        'file_path': '文件路径',
        'teachers_count': '教师数',
        'courses_count': '课程数',
        'schedules_count': '排课数',
        'summary': '摘要',
        'details': '详情',
        'status': '状态',
        'message': '消息',
        'time_slot_name': '时间段',
        'location_name': '地点'
    };
    return dict[field] || field;
};

const paginationConfig = computed(() => ({
    page: currentPage.value,
    pageSize: perPage.value,
    itemCount: totalEntries.value,
    showSizePicker: true,
    pageSizes: [10, 20, 50],
    onUpdatePage: (page) => {
        currentPage.value = page;
        loadAuditLogs();
    },
    onUpdatePageSize: (pageSize) => {
        perPage.value = pageSize;
        currentPage.value = 1;
        loadAuditLogs();
    },
    prefix: () => `共 ${totalEntries.value} 条记录`
}));

const formatLocalDate = (timestamp) => {
    const date = new Date(timestamp);
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
};

const loadAuditLogs = async () => {
    loading.value = true;
    try {
        const filters = {};

        if (filterUsername.value) {
            filters.username = filterUsername.value;
        }

        if (filterActionType.value) {
            filters.actionType = filterActionType.value;
        }

        if (filterDateRange.value && filterDateRange.value.length === 2) {
            const [start, end] = filterDateRange.value;
            filters.dateFrom = formatLocalDate(start);
            filters.dateTo = formatLocalDate(end);
        }

        const result = await invoke('list_audit_logs', {
            page: currentPage.value,
            perPage: perPage.value,
            filters: Object.keys(filters).length > 0 ? filters : null,
            sessionId: authStore.sessionId
        });

        auditLogs.value = result.entries;
        totalEntries.value = result.pagination.totalEntries;
        totalPages.value = result.pagination.totalPages;

    } catch (error) {
        console.error('Failed to load audit logs:', error);
        message.error(`加载审计日志失败: ${error}`);
    } finally {
        loading.value = false;
    }
};

onMounted(async () => {
    loadAuditLogs();
    listeners.push(await listen('commit-completed', () => {
        loadAuditLogs();
    }));

    listeners.push(await listen('data-reloaded', () => {
        loadAuditLogs();
    }));
});

onUnmounted(() => {
    listeners.forEach(unlisten => unlisten());
    listeners.length = 0;
});
</script>

<style scoped>
.json-viewer {
    background-color: #f5f5f5;
    padding: 12px;
    border-radius: 4px;
    font-family: monospace;
    white-space: pre-wrap;
    word-break: break-all;
    font-size: 12px;
    color: #333;
}
</style>
