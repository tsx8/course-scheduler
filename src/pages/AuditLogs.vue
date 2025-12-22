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
    </n-layout>
</template>

<script setup>
import { ref, computed, h, onMounted } from 'vue';
import {
    NLayout, NLayoutHeader, NLayoutContent, NCard, NFlex, NFormItem, NInput, NSelect,
    NDatePicker, NButton, NDataTable, NText, NH2, NTag, useMessage
} from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '../stores/auth';
import { useDataStore } from '../stores/data';

const authStore = useAuthStore();
const message = useMessage();
const dataStore = useDataStore();

const auditLogs = ref([]);
const loading = ref(false);
const currentPage = ref(1);
const perPage = ref(10);
const totalEntries = ref(0);
const totalPages = ref(0);

// Filters
const filterUsername = ref('');
const filterActionType = ref(null);
const filterDateRange = ref(null);

// Action type options (matching backend enum)
const actionTypeOptions = [
    { label: '登录', value: 'LOGIN' },
    { label: '登出', value: 'LOGOUT' },
    { label: '登录失败', value: 'LOGIN_FAILED' },
    { label: '创建用户', value: 'USER_CREATED' },
    { label: '自动创建用户', value: 'AUTO_USER_CREATED' },
    { label: '角色变更', value: 'USER_UPDATED' },
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
            else if (row.action_type.includes('CREATE')) type = 'info';
            else if (row.action_type.includes('UPDATE') || row.action_type.includes('CHANGED')) type = 'warning';

            return h(NTag, { type }, { default: () => label });
        }
    },
    {
        title: '目标表',
        key: 'target_table',
        width: 120,
        render: (row) => row.target_table || '-'
    },
    {
        title: '详细信息',
        key: 'change_details',
        render: (row) => {
            if (!row.change_details) return h(NText, { depth: 3 }, { default: () => '无详情' });

            try {
                const rawDetails = JSON.parse(row.change_details);

                let changes = [];
                let contextInfo = null;

                if (Array.isArray(rawDetails)) {
                    changes = rawDetails;
                } else if (typeof rawDetails === 'object') {
                    changes = rawDetails.changes || [];
                    if (rawDetails.target_username) {
                        contextInfo = rawDetails.target_username;
                    } else if (rawDetails.teacher_name) {
                        contextInfo = rawDetails.teacher_name;
                    } else if (rawDetails.username) {
                        contextInfo = rawDetails.username;
                    }
                }

                const renderList = [];

                if (contextInfo) {
                    renderList.push(
                        h(NFlex, { align: 'center', size: 4, style: 'margin-bottom: 6px; gap: 8px' }, {
                            default: () => [
                                h(NTag, { size: 'tiny', bordered: false }, { default: () => '操作对象' }),
                                h(NText, { strong: true }, { default: () => contextInfo })
                            ]
                        })
                    );
                }

                if (Array.isArray(changes) && changes.length > 0) {
                    const changesNodes = changes.map(item => h('div', {
                        style: 'font-size: 12px; display: flex; align-items: center; gap: 8px;'
                    }, [
                        h(NTag, { size: 'tiny', bordered: false, type: 'info' }, {
                            default: () => translateField(item.field)
                        }),
                        h(NText, { delete: true, depth: 3 }, {
                            default: () => translateValue(item.field, item.old)
                        }),
                        h(NText, { strong: true, depth: 2 }, { default: () => '→' }),
                        h(NText, { type: 'success', strong: true }, {
                            default: () => translateValue(item.field, item.new)
                        })
                    ]));

                    renderList.push(h(NFlex, { vertical: true, size: 4 }, { default: () => changesNodes }));
                } else if (!contextInfo) {
                    return h('pre', { style: 'font-size: 10px; margin: 0; white-space: pre-wrap;' }, JSON.stringify(rawDetails, null, 2));
                }

                return h('div', null, renderList);

            } catch (e) {
                console.warn('JSON parse error', e);
                return row.change_details;
            }
        }
    }
];

const translateValue = (field, value) => {
    if (value === null || value === undefined || value === '' || value === 'null') {
        return '无';
    }

    let result = String(value);

    try {
        switch (field) {
            case 'role_id':
                const role = dataStore.roles.find(r => r.id === value);
                result = role ? role.name : value;
                break;
            case 'teacher_id':
                const teacher = dataStore.teachers.find(t => t.id === value);
                result = teacher ? teacher.name : value;
                break;
            case 'campus_id':
                const campus = dataStore.campuses.find(c => c.id === value);
                result = campus ? campus.name : value;
                break;
            case 'venue_id':
                const venue = dataStore.venues.find(v => v.id === value);
                result = venue ? venue.name : value;
                break;
            case 'is_only_shahe':
                result = (value === true || value === 1 || value === 'true') ? '仅沙河' : '全校区';
                break;
        }
    } catch (e) {
        console.warn('Translation error:', e);
    }

    return String(result);
};

const translateField = (field) => {
    const dict = {
        'role_id': '权限角色',
        'teacher_id': '关联教师',
        'name': '名称',
        'max_teaching_hours': '最大学时',
        'is_only_shahe': '校区限制',
        'venue_id': '场地',
        'campus_id': '校区'
    };
    return dict[field] || field;
};

// Pagination config
const paginationConfig = computed(() => ({
    page: currentPage.value,
    pageSize: perPage.value,
    itemCount: totalEntries.value,
    // pageCount: totalPages.value,
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

// Load audit logs from backend
const loadAuditLogs = async () => {
    loading.value = true;
    try {
        // Build filters object
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

const handleFilterChange = () => {
    // Reset to first page when filters change
    currentPage.value = 1;
};

onMounted(() => {
    loadAuditLogs();
});
</script>

<style scoped>
/* Add any custom styles here */
</style>
