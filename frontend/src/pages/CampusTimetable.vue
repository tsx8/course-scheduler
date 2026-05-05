<script setup>
import { computed, h, ref, watch } from 'vue';
import { useDataStore } from '../stores/data';
import { useScheduleDragStore } from '../stores/scheduleDrag';
import { NSelect, NDataTable, NFlex, NH2, NEmpty, NInputNumber, NButton, NIcon, NModal, NTag, useMessage } from 'naive-ui';
import { DownloadOutline as ExportIcon } from '@vicons/ionicons5';
import ScheduleCard from '../components/ScheduleCard.vue';
import ScheduleDropCell from '../components/ScheduleDropCell.vue';
import ScheduleDetailDrawer from '../components/ScheduleDetailDrawer.vue';

const dataStore = useDataStore();
const scheduleDrag = useScheduleDragStore();
const message = useMessage();

const selectedCampusId = computed({
    get: () => dataStore.selectedCampusIdForCampusView,
    set: (val) => { dataStore.selectedCampusIdForCampusView = val; }
});

const selectedVenueIds = computed({
    get: () => dataStore.selectedVenueIdsForCampusView,
    set: (val) => { dataStore.selectedVenueIdsForCampusView = Array.isArray(val) ? val : []; }
});

const selectedTeacherIds = computed({
    get: () => dataStore.selectedTeacherIdsForCampusView,
    set: (val) => { dataStore.selectedTeacherIdsForCampusView = Array.isArray(val) ? val : []; }
});

const campusOptions = computed(() => dataStore.campusOptions);
const timeSlots = computed(() => dataStore.time);
const days = computed(() => dataStore.day);
const conflictModal = ref({
    show: false,
    sourceScheduleId: null,
    targetScheduleId: null,
    targetPlacement: null,
});
const detailDrawer = ref({
    show: false,
    title: '排课详情',
    issues: [],
});


const venueOptions = computed(() => {
    if (!selectedCampusId.value) return [];
    return dataStore.venueOptionsByCampus(selectedCampusId.value);
});
const teacherOptions = computed(() => {
    if (!selectedCampusId.value) return [];
    const teacherIds = new Set(
        dataStore.activeScheduledClasses
            .filter(schedule => schedule.campus_id === selectedCampusId.value)
            .map(schedule => schedule.teacher_id)
    );

    return dataStore.teachers
        .filter(teacher => teacherIds.has(teacher.id))
        .map(teacher => ({ label: teacher.name, value: teacher.id }));
});


watch(selectedCampusId, () => {
    selectedVenueIds.value = [];
    selectedTeacherIds.value = [];
});

const scheduleMap = computed(() => dataStore.getScheduledClassesByCampus);
const selectedCampus = computed(() => {
    if (!selectedCampusId.value) return null;
    return dataStore.campuses.find(campus => campus.id === selectedCampusId.value) || null;
});

const selectedCampusActiveSchedules = computed(() => {
    if (!selectedCampusId.value) return [];
    return dataStore.activeScheduledClasses.filter(schedule => schedule.campus_id === selectedCampusId.value);
});

const selectedCampusScheduleIds = computed(() => new Set(selectedCampusActiveSchedules.value.map(schedule => schedule.id)));

const campusIssues = computed(() => {
    if (!selectedCampusId.value) return [];
    const scheduleIds = selectedCampusScheduleIds.value;
    return dataStore.scheduleIssues.filter(issue => {
        if (issue.campus_id === selectedCampusId.value) return true;
        return issue.schedule_ids?.some(scheduleId => scheduleIds.has(scheduleId));
    });
});

const campusIssueCounts = computed(() => campusIssues.value.reduce((counts, issue) => {
    counts[issue.severity] = (counts[issue.severity] || 0) + 1;
    return counts;
}, { error: 0, warning: 0 }));

const campusLoadStats = computed(() => {
    if (!selectedCampusId.value) return { actual: 0, expected: 0, excess: 0 };

    const actualByCell = new Map();
    selectedCampusActiveSchedules.value.forEach(schedule => {
        const key = `${schedule.day_id}-${schedule.time_id}`;
        actualByCell.set(key, (actualByCell.get(key) || 0) + 1);
    });

    const expectedByCell = new Map();
    dataStore.scheduleDensity
        .filter(density => density.campus_id === selectedCampusId.value)
        .forEach(density => {
            const key = `${density.day_id}-${density.time_id}`;
            expectedByCell.set(key, Number(density.count || 0));
        });

    const keys = new Set([...actualByCell.keys(), ...expectedByCell.keys()]);
    let expected = 0;
    let excess = 0;
    keys.forEach(key => {
        const actualCount = actualByCell.get(key) || 0;
        const expectedCount = expectedByCell.get(key) || 0;
        expected += expectedCount;
        excess += Math.max(0, actualCount - expectedCount);
    });

    return {
        actual: selectedCampusActiveSchedules.value.length,
        expected,
        excess,
    };
});

const campusSummaryItems = computed(() => [
    { label: '总量', value: `${campusLoadStats.value.actual}/${campusLoadStats.value.expected}` },
    { label: '超量', value: campusLoadStats.value.excess, type: campusLoadStats.value.excess > 0 ? 'warning' : null },
    {
        label: '异常',
        value: `${campusIssueCounts.value.error} 冲突 / ${campusIssueCounts.value.warning} 风险`,
        type: campusIssueCounts.value.error > 0 ? 'error' : (campusIssueCounts.value.warning > 0 ? 'warning' : null),
    },
]);


const tableData = computed(() => {
    if (!selectedCampusId.value) return [];
    return timeSlots.value.map(time => {
        const row = {
            key: time.id,
            time_slot: time.value
        };
        days.value.forEach(day => {
            const mapKey = `${day.id}-${time.id}`;
            row[day.id] = sortCellSchedules(scheduleMap.value.get(mapKey) || []);
        });
        return row;
    });
});

const sortCellSchedules = (schedules) => {
    return schedules.slice().sort((left, right) => {
        const leftSchedule = left.schedule;
        const rightSchedule = right.schedule;
        return `${leftSchedule.venue_id || ''}-${leftSchedule.teacher_id || ''}-${leftSchedule.course_id || ''}-${leftSchedule.id}`
            .localeCompare(`${rightSchedule.venue_id || ''}-${rightSchedule.teacher_id || ''}-${rightSchedule.course_id || ''}-${rightSchedule.id}`);
    });
};

const cellTarget = (dayId, timeId) => {
    const target = {
        type: 'campus',
        campus_id: selectedCampusId.value,
        day_id: dayId,
        time_id: timeId,
    };

    if (selectedVenueIds.value.length === 1) {
        target.venue_id = selectedVenueIds.value[0];
    }

    return target;
};

const campusIssueKey = (target) => `${target.campus_id}-${target.day_id}-${target.time_id}`;
const cellIssues = (target) => dataStore.issuesByCampusCell.get(campusIssueKey(target)) || [];
const issueTouchesSchedules = (issue, scheduleIds) => issue.schedule_ids?.some(scheduleId => scheduleIds.has(scheduleId));
const findScheduleById = (scheduleId) => dataStore.scheduledClasses.find(schedule => schedule.id === scheduleId) || null;
const collectScheduleIds = (target = {}) => {
    const ids = [
        ...(Array.isArray(target.schedule_ids) ? target.schedule_ids : []),
        target.schedule_id,
        target.scheduleId,
    ].filter(Boolean);
    return [...new Set(ids)];
};
const focusScheduleIdSet = (focus) => new Set(collectScheduleIds(focus));
const fieldMatches = (expected, actual) => !expected || expected === actual;
const scheduleMatchesFocus = (schedule, focus) => {
    if (!schedule || !focus) return false;
    const scheduleIds = focusScheduleIdSet(focus);
    if (scheduleIds.has(schedule.id)) return true;
    if (focus.type && focus.type !== 'campus') return false;

    const hasScheduleScope = Boolean(focus.teacher_id || focus.campus_id || focus.venue_id || focus.day_id || focus.time_id);
    return hasScheduleScope
        && fieldMatches(focus.teacher_id, schedule.teacher_id)
        && fieldMatches(focus.campus_id, schedule.campus_id)
        && fieldMatches(focus.venue_id, schedule.venue_id)
        && fieldMatches(focus.day_id, schedule.day_id)
        && fieldMatches(focus.time_id, schedule.time_id);
};
const targetHasFocusedSchedule = (target, focus) => {
    const scheduleIds = focusScheduleIdSet(focus);
    if (scheduleIds.size === 0 || !Array.isArray(target.schedule_ids)) return false;
    return target.schedule_ids.some(scheduleId => scheduleIds.has(scheduleId));
};
const activeScheduleId = () => scheduleDrag.draggedScheduleId || null;
const isFocusedSchedule = (schedule) => scheduleMatchesFocus(schedule, dataStore.focusedScheduleTarget);
const isFocusedCell = (target) => {
    const focus = dataStore.focusedScheduleTarget;
    if (!focus) return false;
    if (focus.type && focus.type !== 'campus') return false;
    if (!fieldMatches(focus.campus_id, target.campus_id)) return false;
    if (!fieldMatches(focus.day_id, target.day_id)) return false;
    if (!fieldMatches(focus.time_id, target.time_id)) return false;
    if (focus.venue_id && target.venue_id && focus.venue_id !== target.venue_id) return false;
    if (targetHasFocusedSchedule(target, focus)) return true;
    return Boolean(focus.day_id || focus.time_id || focus.venue_id);
};
const visualCellIssues = (target, schedulesInCell = []) => {
    const issues = cellIssues(target);
    const hasActiveScheduleFilter = selectedVenueIds.value.length > 0 || selectedTeacherIds.value.length > 0;
    if (!hasActiveScheduleFilter) return issues;

    const visibleScheduleIds = new Set(schedulesInCell.map(({ schedule }) => schedule.id));
    return issues.filter(issue => {
        if (issue.venue_id && selectedVenueIds.value.length > 0 && !selectedVenueIds.value.includes(issue.venue_id)) return false;
        return issueTouchesSchedules(issue, visibleScheduleIds);
    });
};
const scheduleIssues = (scheduleId) => dataStore.issuesByScheduleId.get(scheduleId) || [];
const campusTableOnlyIssueCategories = new Set([
    'teacher_day_campus_conflict',
    'teacher_hours_warning',
]);
const shouldHighlightCampusIssue = (issue) => !campusTableOnlyIssueCategories.has(issue.category);
const isHoverTarget = (target) => {
    const hoverTarget = scheduleDrag.hoverTarget;
    return hoverTarget?.type === 'campus'
        && hoverTarget.campus_id === target.campus_id
        && hoverTarget.day_id === target.day_id
        && hoverTarget.time_id === target.time_id
        && hoverTarget.venue_id === target.venue_id;
};
const findById = (records, id) => records.find(record => record.id === id);
const dayName = (dayId) => findById(dataStore.day, dayId)?.value || '未知日期';
const timeName = (timeId) => findById(dataStore.time, timeId)?.value || '未知时间';
const venueName = (venueId) => findById(dataStore.venues, venueId)?.name || '全部场地';
const venueScopeName = (target) => {
    if (target.venue_id) return venueName(target.venue_id);
    if (selectedVenueIds.value.length === 0) return '全部场地';
    return `${selectedVenueIds.value.length}个场地`;
};
const dedupeIssues = (issues) => {
    const issueMap = new Map();
    issues.filter(Boolean).forEach(issue => issueMap.set(issue.id, issue));
    return [...issueMap.values()];
};

const cellSummaryItems = (target, actualCount, issues = cellIssues(target)) => {
    const expectedCount = dataStore.getExpectedCountForCampusCell(target.campus_id, target.day_id, target.time_id);
    const issueCounts = issues.reduce((counts, issue) => {
        counts[issue.severity] = (counts[issue.severity] || 0) + 1;
        return counts;
    }, { error: 0, warning: 0 });

    return [
        { label: '期望 / 实际', value: `${expectedCount}/${actualCount}` },
        { label: '缺口', value: Math.max(0, expectedCount - actualCount), type: expectedCount > actualCount ? 'warning' : null },
        { label: '超量', value: Math.max(0, actualCount - expectedCount), type: actualCount > expectedCount ? 'warning' : null },
        {
            label: '异常',
            value: `${issueCounts.error} 冲突 / ${issueCounts.warning} 风险`,
            type: issueCounts.error > 0 ? 'error' : (issueCounts.warning > 0 ? 'warning' : null),
        },
    ];
};

const openCampusDetail = () => {
    if (!selectedCampus.value) return;
    detailDrawer.value = {
        show: true,
        title: `${selectedCampus.value.name} 排课详情`,
        issues: campusIssues.value,
    };
};

const openCellDetail = (target, schedulesInCell = []) => {
    const schedules = schedulesInCell.map(item => item.schedule);
    const visualIssues = visualCellIssues(target, schedulesInCell);
    const issues = dedupeIssues([
        ...visualIssues,
        ...schedules.flatMap(schedule => scheduleIssues(schedule.id)),
    ]);

    dataStore.setScheduleFocus({ type: 'campus', ...target });
    detailDrawer.value = {
        show: true,
        title: `${selectedCampus.value?.name || '校区'} · ${dayName(target.day_id)} ${timeName(target.time_id)} · ${venueScopeName(target)}`,
        issues,
    };
};

const buildCampusIssueFocus = (issue) => {
    const rawFocus = issue.focus || {};
    const scheduleIds = [...new Set([...collectScheduleIds(issue), ...collectScheduleIds(rawFocus)])];
    const schedules = scheduleIds.map(findScheduleById).filter(Boolean);
    const currentCampusSchedule = schedules.find(schedule => schedule.campus_id === selectedCampusId.value);
    const campusId = rawFocus.campus_id
        || issue.campus_id
        || currentCampusSchedule?.campus_id
        || schedules[0]?.campus_id
        || selectedCampusId.value
        || null;

    return {
        type: 'campus',
        ...rawFocus,
        schedule_ids: scheduleIds,
        teacher_id: rawFocus.teacher_id ?? issue.teacher_id ?? null,
        course_id: rawFocus.course_id ?? issue.course_id ?? null,
        campus_id: campusId,
        venue_id: rawFocus.venue_id ?? issue.venue_id ?? null,
        day_id: rawFocus.day_id ?? issue.day_id ?? null,
        time_id: rawFocus.time_id ?? issue.time_id ?? null,
    };
};

const handleDetailLocate = (issue) => {
    const focus = buildCampusIssueFocus(issue);
    if (focus.campus_id) selectedCampusId.value = focus.campus_id;
    selectedTeacherIds.value = focus.teacher_id ? [focus.teacher_id] : [];
    if (focus.venue_id) selectedVenueIds.value = [focus.venue_id];
    else if (focus.campus_id) selectedVenueIds.value = [];
    if (shouldHighlightCampusIssue(issue)) {
        dataStore.setScheduleFocus(focus);
    } else {
        dataStore.clearScheduleFocus();
    }
    detailDrawer.value.show = false;
};


const handlePointerDragStart = ({ schedule, event }) => {
    event.preventDefault();
    const source = {
        type: 'campus',
        campus_id: schedule.campus_id,
        day_id: schedule.day_id,
        time_id: schedule.time_id,
    };

    if (schedule.venue_id) {
        source.venue_id = schedule.venue_id;
    }

    scheduleDrag.startDrag({ schedule, source });
};

const handleCellPointerEnter = ({ target }) => {
    if (!activeScheduleId()) return;
    scheduleDrag.setHoverTarget(target);
};

const handleCellPointerLeave = () => {
    if (!activeScheduleId()) return;
    scheduleDrag.setHoverTarget(null);
};

const findVenueById = (venueId) => dataStore.venues.find(venue => venue.id === venueId) || null;
const installVenueIdForTarget = (sourceSchedule, target) => target.venue_id ?? sourceSchedule?.venue_id ?? null;
const canInstallWithExistingVenue = (sourceSchedule, target) => {
    if (!sourceSchedule || target.venue_id) return true;
    if (!sourceSchedule.venue_id) return false;
    return findVenueById(sourceSchedule.venue_id)?.campus_id === target.campus_id;
};
const conflictsWithCampusTargetSchedule = (sourceSchedule, targetSchedule, target) => {
    if (!sourceSchedule || !targetSchedule || sourceSchedule.id === targetSchedule.id) return false;
    const targetVenueId = installVenueIdForTarget(sourceSchedule, target);
    const sameTeacherAtTime = sourceSchedule.teacher_id === targetSchedule.teacher_id;
    const sameVenueAtTime = Boolean(targetVenueId) && targetVenueId === targetSchedule.venue_id;
    return sameTeacherAtTime || sameVenueAtTime;
};

const installDraggedSchedule = (sourceScheduleId, target) => {
    const sourceSchedule = findScheduleById(sourceScheduleId);
    if (!sourceSchedule) return;

    if (!canInstallWithExistingVenue(sourceSchedule, target)) {
        scheduleDrag.endDrag();
        message.warning('请先选择具体场地，再把排课安装到该校区');
        return;
    }

    const installedSchedule = dataStore.installSchedule(sourceScheduleId, target);
    scheduleDrag.endDrag();

    if (!installedSchedule) {
        message.error('无法安装排课');
    }
};

const handleBlankAreaDrop = ({ target }) => {
    const sourceScheduleId = activeScheduleId();
    if (!sourceScheduleId) return;

    installDraggedSchedule(sourceScheduleId, target);
};

const conflictTargetPlacement = (sourceSchedule, targetSchedule, target) => ({
    teacher_id: sourceSchedule?.teacher_id,
    campus_id: target?.campus_id ?? targetSchedule?.campus_id,
    venue_id: targetSchedule?.venue_id ?? target?.venue_id ?? sourceSchedule?.venue_id,
    day_id: target?.day_id ?? targetSchedule?.day_id,
    time_id: target?.time_id ?? targetSchedule?.time_id,
});

const openConflictModal = (sourceScheduleId, targetScheduleId, targetPlacement = null) => {
    if (!sourceScheduleId || !targetScheduleId) return;
    if (sourceScheduleId === targetScheduleId) {
        scheduleDrag.endDrag();
        return;
    }

    conflictModal.value = {
        show: true,
        sourceScheduleId,
        targetScheduleId,
        targetPlacement,
    };
};

const handleCardDrop = ({ target, targetScheduleId }) => {
    const sourceScheduleId = activeScheduleId();
    if (!sourceScheduleId) return;

    const sourceSchedule = findScheduleById(sourceScheduleId);
    const targetSchedule = findScheduleById(targetScheduleId || target.schedule_ids?.[0]);
    if (conflictsWithCampusTargetSchedule(sourceSchedule, targetSchedule, target)) {
        openConflictModal(sourceScheduleId, targetSchedule.id, conflictTargetPlacement(sourceSchedule, targetSchedule, target));
        return;
    }

    installDraggedSchedule(sourceScheduleId, target);
};

const closeConflictModal = () => {
    conflictModal.value = {
        show: false,
        sourceScheduleId: null,
        targetScheduleId: null,
        targetPlacement: null,
    };
    scheduleDrag.cancelDrag();
};

const applyConflictChoice = (choice) => {
    const { sourceScheduleId, targetScheduleId, targetPlacement } = conflictModal.value;
    let result = null;

    if (choice === 'replace') {
        result = dataStore.replaceSchedule(sourceScheduleId, targetScheduleId, targetPlacement || {});
    } else if (choice === 'swap') {
        result = dataStore.swapSchedules(sourceScheduleId, targetScheduleId, ['campus_id', 'venue_id', 'day_id', 'time_id']);
    } else if (choice === 'displace') {
        result = dataStore.displaceSchedule(sourceScheduleId, targetScheduleId, targetPlacement || {});
    }

    conflictModal.value = {
        show: false,
        sourceScheduleId: null,
        targetScheduleId: null,
        targetPlacement: null,
    };
    scheduleDrag.endDrag();

    if (result) {
        message.success('排课已调整');
    } else {
        message.error('无法调整排课');
    }
};

const renderCounterNode = (target, actualCount) => {
    const expectedCount = dataStore.getExpectedCountForCampusCell(target.campus_id, target.day_id, target.time_id);
    const isOverbooked = actualCount > expectedCount && expectedCount > 0;

    return h('div', {
        class: 'campus-cell-counter',
        style: {
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '2px',
            marginBottom: '6px',
            minWidth: '0',
            whiteSpace: 'nowrap',
        }
    }, [
        h('span', {
            class: [
                'campus-cell-counter__label',
                isOverbooked ? 'campus-cell-counter__label--overbooked' : null,
            ]
        }, `容量：${actualCount}/`),
        h(NInputNumber, {
            value: expectedCount,
            'onUpdate:value': (value) => {
                dataStore.updateCampusScheduleDensity(target.campus_id, target.day_id, target.time_id, value);
            },
            size: 'tiny',
            min: 0,
            precision: 0,
            showButton: false,
            class: 'campus-cell-counter__input',
            style: {
                width: '36px',
                flex: '0 0 36px',
                minWidth: '36px',
            },
            'aria-label': '期望容量',
        })
    ]);
};

const renderScheduleCard = (schedule) => h(ScheduleCard, {
    schedule,
    context: 'campus',
    issues: scheduleIssues(schedule.id),
    focused: isFocusedSchedule(schedule),
    actions: { edit: false, delete: false },
    onPointerDragStart: handlePointerDragStart,
    onLockToggle: (targetSchedule) => dataStore.setScheduleLocked(targetSchedule.id, !targetSchedule.is_locked),
    onStage: (targetSchedule) => dataStore.stageSchedule(targetSchedule.id),
});

const renderCell = (dayId, timeId, schedulesInCell) => {
    const target = {
        ...cellTarget(dayId, timeId),
        schedule_ids: schedulesInCell.map(({ schedule }) => schedule.id),
    };
    const cards = schedulesInCell.map(({ schedule }) => renderScheduleCard(schedule));
    const visualIssues = visualCellIssues(target, schedulesInCell);

    return h(ScheduleDropCell, {
        target,
        issues: visualIssues,
        hover: isHoverTarget(target),
        focused: isFocusedCell(target),
        canDrop: Boolean(activeScheduleId()),
        occupied: false,
        showIssueBadge: false,
        showBlankArea: false,
        onPointerEnter: handleCellPointerEnter,
        onPointerLeave: handleCellPointerLeave,
        onBlankAreaDrop: handleBlankAreaDrop,
        onCardDrop: handleCardDrop,
        onIssueClick: () => openCellDetail(target, schedulesInCell),
    }, {
        default: () => selectedVenueIds.value.length > 0
            ? cards
            : [renderCounterNode(target, schedulesInCell.length), ...cards],
    });
};

const columns = computed(() => {
    if (!selectedCampusId.value) return [];
    const dayColumns = days.value.map(day => ({
        title: day.value,
        key: day.id,
        align: 'center',
        cellProps: () => ({
            style: {
                verticalAlign: 'top',
                padding: '8px'
            }
        }),
        render(row) {
            const timeId = row.key;
            return renderCell(day.id, timeId, row[day.id]);
        }
    }));

    return [
        {
            title: '时间',
            key: 'time_slot',
            align: 'center',
            fixed: 'left',
            resizable: true,
            width: '10%',
            render(row) {
                return h('div', { style: 'font-weight: bold;' }, row.time_slot);
            }
        },
        ...dayColumns
    ];
});

const handleExportToCsv = () => {
    if (!selectedCampusId.value) {
        message.warning('请先选择一个校区');
        return;
    }

    const headers = ['时间', ...days.value.map(d => d.value)];
    const rows = tableData.value.map(row => {
        const rowData = [row.time_slot];
        days.value.forEach(day => {
            const schedules = row[day.id];
            if (schedules.length === 0) {
                rowData.push('');
            } else {
                const cellContent = schedules.map(({ schedule, teacher }) => {
                    const course = dataStore.courses.find(c => c.id === schedule.course_id);
                    const venue = dataStore.venues.find(v => v.id === schedule.venue_id);

                    const courseName = course?.name || '未知课程';
                    const teacherName = teacher?.name || '未知教师';
                    const venueName = venue?.name || '未知场地';

                    return `${courseName} (${teacherName} - ${venueName})`;
                }).join('\n');

                rowData.push(`"${cellContent.replace(/"/g, '""')}"`);
            }
        });
        return rowData.join(',');
    });

    const csvContent = [headers.join(','), ...rows].join('\n');

    const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' });

    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);
    link.setAttribute('href', url);

    const campusName = campusOptions.value.find(c => c.value === selectedCampusId.value)?.label || '课表';
    const venueNames = selectedVenueIds.value
        .map(venueId => dataStore.venues.find(venue => venue.id === venueId)?.name)
        .filter(Boolean);
    const venueNamePart = venueNames.length === 0
        ? ''
        : `_${venueNames.length === 1 ? venueNames[0] : `${venueNames.length}个场地`}`;
    const teacherNames = selectedTeacherIds.value
        .map(teacherId => dataStore.teachers.find(teacher => teacher.id === teacherId)?.name)
        .filter(Boolean);
    const teacherNamePart = teacherNames.length === 0
        ? ''
        : `_${teacherNames.length === 1 ? teacherNames[0] : `${teacherNames.length}位教师`}`;
    const fileSuffix = venueNames.length > 0 || teacherNames.length > 0 ? '课表' : '总课表';
    const fileName = `${campusName}${venueNamePart}${teacherNamePart}_${fileSuffix}.csv`;

    link.setAttribute('download', fileName);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    message.success('CSV文件已开始下载');
};

watch(() => dataStore.campuses, (newCampuses) => {
    if (selectedCampusId.value && !newCampuses.find(c => c.id === selectedCampusId.value)) {
        selectedCampusId.value = null;
    }
}, { deep: true });
</script>

<template>
    <section class="timetable-page">
        <header class="timetable-page__header">
            <n-flex justify="space-between" align="center" :wrap="false">
                <n-h2 style="margin: 0; white-space: nowrap; flex-shrink: 0;">校区总课表</n-h2>
                <n-flex :wrap="false" align="center" style="flex-shrink: 1; overflow: hidden;">
                    <n-select v-model:value="selectedCampusId" placeholder="请选择校区" :options="campusOptions" clearable
                        style="width: 150px" />
                    <n-select v-model:value="selectedVenueIds" multiple max-tag-count="responsive" placeholder="选择场地" :options="venueOptions" clearable
                        :disabled="!selectedCampusId" style="width: 180px" />
                    <n-select v-model:value="selectedTeacherIds" multiple max-tag-count="responsive" placeholder="选择教师" :options="teacherOptions" clearable
                        :disabled="!selectedCampusId" style="width: 180px" />
                    <n-button type="primary" @click="handleExportToCsv" :disabled="!selectedCampusId">
                        <template #icon><n-icon :component="ExportIcon" /></template>
                        导出CSV
                    </n-button>
                </n-flex>
            </n-flex>
        </header>

        <main class="timetable-page__content">
            <div v-if="selectedCampus" class="summary-bar" role="button" tabindex="0" @click="openCampusDetail" @keyup.enter="openCampusDetail" @keyup.space="openCampusDetail">
                <div v-for="item in campusSummaryItems" :key="item.label" class="summary-bar__item" :class="item.type ? `summary-bar__item--${item.type}` : null">
                    <span>{{ item.label }}</span>
                    <strong>{{ item.value }}</strong>
                </div>
                <n-button class="summary-bar__action" tertiary size="small" type="primary" @click.stop="openCampusDetail">详情</n-button>
            </div>

            <div v-if="selectedCampusId" class="timetable-scroll">
                <n-data-table :columns="columns" :data="tableData" :bordered="true"
                    :single-line="false" style="width: 100%;" />
            </div>
            <n-flex v-else justify="center" align="center" class="timetable-empty">
                <n-empty description="请先选择一个校区以查看课表" size="huge" />
            </n-flex>
        </main>

        <n-modal v-model:show="conflictModal.show" preset="dialog" title="目标位置已有排课" @mask-click="closeConflictModal">
            <p>请选择如何处理目标位置已有排课。</p>
            <template #action>
                <n-flex justify="end" :wrap="false">
                    <n-button @click="closeConflictModal">取消</n-button>
                    <n-button type="warning" @click="applyConflictChoice('displace')">换下</n-button>
                    <n-button type="info" @click="applyConflictChoice('swap')">交换</n-button>
                    <n-button type="primary" @click="applyConflictChoice('replace')">覆盖</n-button>
                </n-flex>
            </template>
        </n-modal>

        <ScheduleDetailDrawer
            v-model:show="detailDrawer.show"
            :title="detailDrawer.title"
            :issues="detailDrawer.issues"
            @locate="handleDetailLocate"
        />
    </section>
</template>


<style scoped>
.timetable-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
}

.timetable-page__header {
    flex: 0 0 auto;
    padding: 12px 24px;
    border-bottom: 1px solid #efeff5;
}

.timetable-page__content {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    padding: 24px;
    overflow: hidden;
}
.timetable-scroll {
    flex: 0 1 auto;
    min-height: 0;
    max-height: 100%;
    overflow: auto;
}

.campus-cell-counter {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-bottom: 6px;
    min-width: 0;
}

.campus-cell-counter__label {
    flex: 0 0 auto;
    color: #606266;
    white-space: nowrap;
    font-size: 12px;
    line-height: 1;
}

.campus-cell-counter__label--overbooked {
    color: #d03050;
    font-weight: 600;
}

.campus-cell-counter__input {
    width: 42px;
    flex: 0 0 42px;
}

.campus-cell-counter__input :deep(.n-input-wrapper) {
    padding-left: 4px;
    padding-right: 4px;
}


.timetable-empty {
    flex: 1 1 auto;
    min-height: 0;
}

.summary-bar {
    display: flex;
    align-items: center;
    align-self: flex-start;
    flex: 0 0 auto;
    flex-wrap: wrap;
    gap: 6px;
    max-width: 100%;
    margin-bottom: 12px;
    padding: 8px;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    background: #f8fafc;
    cursor: pointer;
}

.summary-bar:focus-visible {
    outline: 2px solid #18a058;
    outline-offset: 2px;
}

.summary-bar__item {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
    padding: 5px 8px;
    border: 1px solid #e5e7eb;
    border-radius: 999px;
    background: #fff;
}

.summary-bar__item span {
    display: block;
    color: #606266;
    font-size: 12px;
    line-height: 1;
}

.summary-bar__item strong {
    display: block;
    color: #303133;
    font-size: 14px;
    line-height: 1;
}

.summary-bar__item--error {
    border-color: #f2b8b5;
    background: #fff6f6;
}

.summary-bar__item--warning {
    border-color: #f3d19e;
    background: #fff8ee;
}

.summary-bar__item--error strong {
    color: #d03050;
}

.summary-bar__item--warning strong {
    color: #d46b08;
}
.summary-bar__action {
    flex: 0 0 auto;
}

</style>