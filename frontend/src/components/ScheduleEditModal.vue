<template>
    <n-modal :show="show" preset="dialog" :title="modalTitle" @update:show="handleShowUpdate">
        <n-form
            ref="formRef"
            :model="formValue"
            :rules="rules"
            label-placement="left"
            label-width="auto"
            require-mark-placement="right-hanging"
        >
            <n-form-item label="授课课程" path="course_id">
                <n-select
                    v-model:value="formValue.course_id"
                    placeholder="选择课程"
                    :options="courseOptionsForForm"
                    filterable
                    @update:value="handleCourseChange"
                />
            </n-form-item>
            <n-form-item v-if="isCampusMode" label="授课教师" path="teacher_id">
                <n-select
                    v-model:value="formValue.teacher_id"
                    placeholder="选择教师"
                    :options="teacherOptionsForForm"
                    :disabled="!formValue.course_id"
                    filterable
                />
            </n-form-item>
            <n-form-item v-else label="上课校区" path="campus_id">
                <n-select
                    v-model:value="formValue.campus_id"
                    placeholder="选择校区"
                    :options="campusOptionsForForm"
                    :disabled="!formValue.course_id"
                    filterable
                    @update:value="handleCampusChange"
                />
            </n-form-item>
            <n-form-item label="上课场地" path="venue_id">
                <n-select
                    v-model:value="formValue.venue_id"
                    placeholder="选择场地"
                    :options="venueOptionsForForm"
                    :disabled="!formValue.course_id || !resolvedCampusId"
                    filterable
                />
            </n-form-item>
        </n-form>
        <template #action>
            <n-button @click="closeModal">取消</n-button>
            <n-button type="primary" @click="handleSubmit">确认</n-button>
        </template>
    </n-modal>
</template>

<script setup>
import { computed, nextTick, ref, watch } from 'vue';
import { NButton, NForm, NFormItem, NModal, NSelect, useMessage } from 'naive-ui';
import { useDataStore } from '../stores/data';

const props = defineProps({
    show: { type: Boolean, default: false },
    mode: {
        type: String,
        default: 'teacher',
        validator: value => ['teacher', 'campus'].includes(value),
    },
    teacherId: { type: String, default: '' },
    campusId: { type: String, default: '' },
    venueId: { type: String, default: '' },
    teacherScopeIds: { type: Array, default: () => [] },
    courseScopeIds: { type: Array, default: () => [] },
    venueScopeIds: { type: Array, default: () => [] },
    dayId: { type: String, default: '' },
    timeId: { type: String, default: '' },
    schedule: { type: Object, default: null },
});

const emit = defineEmits(['update:show', 'saved']);

const dataStore = useDataStore();
const message = useMessage();
const formRef = ref(null);
const formValue = ref({});

const isCampusMode = computed(() => props.mode === 'campus');
const isEditMode = computed(() => Boolean(props.schedule?.id));
const modalTitle = computed(() => (isEditMode.value ? '编辑排课' : '新增排课'));
const fixedVenueId = computed(() => (isCampusMode.value ? props.venueId : ''));
const scopedVenueIds = computed(() => {
    if (fixedVenueId.value) return [fixedVenueId.value];
    return Array.isArray(props.venueScopeIds) ? props.venueScopeIds.filter(Boolean) : [];
});
const resolvedCampusId = computed(() => (
    isCampusMode.value ? props.campusId : formValue.value.campus_id
));

const filterOptionsByScope = (options, scopeIds) => {
    const ids = Array.isArray(scopeIds) ? scopeIds.filter(Boolean) : [];
    if (ids.length === 0) return options;
    const allowedIds = new Set(ids);
    return options.filter(option => allowedIds.has(option.value));
};

const courseOptionsForForm = computed(() => {
    if (isCampusMode.value) {
        const options = dataStore.campusCourseOptions(props.campusId, scopedVenueIds.value);
        return filterOptionsByScope(options, props.courseScopeIds);
    }
    return dataStore.teacherCourseOptions(props.teacherId);
});

const teacherOptionsForForm = computed(() => {
    if (!isCampusMode.value || !formValue.value.course_id) return [];
    const options = dataStore.courseTeacherOptions(formValue.value.course_id, props.campusId);
    return filterOptionsByScope(options, props.teacherScopeIds);
});

const campusOptionsForForm = computed(() => {
    if (isCampusMode.value || !formValue.value.course_id) return [];
    return dataStore.courseCampusOptions(formValue.value.course_id, props.teacherId);
});

const venueOptionsForForm = computed(() => {
    if (!formValue.value.course_id || !resolvedCampusId.value) return [];
    const options = dataStore.courseVenueOptions(formValue.value.course_id, resolvedCampusId.value);
    return filterOptionsByScope(options, scopedVenueIds.value);
});

const rules = computed(() => ({
    course_id: { required: true, message: '请选择授课课程', trigger: 'change' },
    ...(isCampusMode.value
        ? { teacher_id: { required: true, message: '请选择授课教师', trigger: 'change' } }
        : { campus_id: { required: true, message: '请选择上课校区', trigger: 'change' } }),
    venue_id: { required: true, message: '请选择上课场地', trigger: 'change' },
}));

const hasOptionValue = (options, value) => {
    return Boolean(value) && options.some(option => option.value === value);
};

const buildInitialForm = () => {
    const schedule = props.schedule || {};
    return {
        ...schedule,
        course_id: schedule.course_id || null,
        teacher_id: isCampusMode.value ? (schedule.teacher_id || null) : (props.teacherId || schedule.teacher_id || null),
        campus_id: isCampusMode.value ? (props.campusId || schedule.campus_id || null) : (schedule.campus_id || null),
        venue_id: schedule.venue_id || null,
    };
};

const syncFormValueWithOptions = () => {
    if (!props.show) return;

    if (formValue.value.course_id && !hasOptionValue(courseOptionsForForm.value, formValue.value.course_id)) {
        formValue.value.course_id = null;
        formValue.value.teacher_id = isCampusMode.value ? null : props.teacherId || null;
        formValue.value.campus_id = isCampusMode.value ? props.campusId || null : null;
        formValue.value.venue_id = null;
        return;
    }

    if (isCampusMode.value) {
        formValue.value.campus_id = props.campusId || null;
        if (formValue.value.teacher_id && !hasOptionValue(teacherOptionsForForm.value, formValue.value.teacher_id)) {
            formValue.value.teacher_id = null;
        }
    } else {
        formValue.value.teacher_id = props.teacherId || null;
        if (formValue.value.campus_id && !hasOptionValue(campusOptionsForForm.value, formValue.value.campus_id)) {
            formValue.value.campus_id = null;
            formValue.value.venue_id = null;
        }
    }

    if (formValue.value.venue_id && !hasOptionValue(venueOptionsForForm.value, formValue.value.venue_id)) {
        formValue.value.venue_id = null;
    }
};

const resetForm = () => {
    formValue.value = buildInitialForm();
    void nextTick(syncFormValueWithOptions);
};

watch(() => [
    props.show,
    props.mode,
    props.teacherId,
    props.campusId,
    props.venueId,
    props.teacherScopeIds,
    props.courseScopeIds,
    props.venueScopeIds,
    props.dayId,
    props.timeId,
    props.schedule,
], ([isOpen]) => {
    if (isOpen) resetForm();
}, { immediate: true });

watch([
    courseOptionsForForm,
    teacherOptionsForForm,
    campusOptionsForForm,
    venueOptionsForForm,
], () => {
    syncFormValueWithOptions();
}, { deep: true });

const closeModal = () => {
    emit('update:show', false);
};

const handleShowUpdate = (value) => {
    emit('update:show', value);
};

const handleCourseChange = () => {
    if (isCampusMode.value) {
        formValue.value.teacher_id = null;
        formValue.value.campus_id = props.campusId || null;
    } else {
        formValue.value.campus_id = null;
        formValue.value.teacher_id = props.teacherId || null;
    }
    formValue.value.venue_id = null;
};

const handleCampusChange = () => {
    formValue.value.venue_id = null;
};

const validateOptionMembership = () => {
    if (!hasOptionValue(courseOptionsForForm.value, formValue.value.course_id)) {
        return '请选择有效的授课课程';
    }
    if (isCampusMode.value && !hasOptionValue(teacherOptionsForForm.value, formValue.value.teacher_id)) {
        return '请选择可在当前校区授课的教师';
    }
    if (!isCampusMode.value && !hasOptionValue(campusOptionsForForm.value, formValue.value.campus_id)) {
        return '请选择教师可授课且课程有场地的校区';
    }
    if (!hasOptionValue(venueOptionsForForm.value, formValue.value.venue_id)) {
        return '请选择课程可使用的场地';
    }
    return null;
};

const handleSubmit = () => {
    if (!props.dayId || !props.timeId || !resolvedCampusId.value) {
        message.error('缺少排课位置，无法保存');
        return;
    }

    formRef.value?.validate((errors) => {
        if (errors) {
            message.error('请填写完整的排课信息');
            return;
        }

        const invalidMessage = validateOptionMembership();
        if (invalidMessage) {
            message.error(invalidMessage);
            return;
        }

        const teacherId = isCampusMode.value ? formValue.value.teacher_id : props.teacherId;
        const scheduleData = {
            ...formValue.value,
            teacher_id: teacherId,
            campus_id: resolvedCampusId.value,
            day_id: props.dayId,
            time_id: props.timeId,
            venue_id: formValue.value.venue_id,
            course_id: formValue.value.course_id,
        };

        if (isEditMode.value) {
            dataStore.updateSchedule(teacherId, scheduleData);
            message.success('排课更新成功');
            emit('saved', { action: 'update', schedule: scheduleData });
        } else {
            dataStore.addSchedule(teacherId, scheduleData);
            message.success('新增排课成功');
            emit('saved', { action: 'create', schedule: scheduleData });
        }

        closeModal();
    });
};
</script>
