import { defineStore } from 'pinia';
import { ref, watch, toRaw, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { v4 as uuidv4 } from 'uuid';
import { useAuthStore } from './auth';

const FIELD_LABELS = {
    name: '名称',
    max_teaching_hours: '最大学时',
    is_only_shahe: '仅沙河校区',
    value: '内容',
    corresponding_hours: '对应学时',
    course_id: '课程',
    venue_id: '场地',
    campus_id: '校区',
    teacher_id: '教师',
    teacher_name: '教师',
    day_id: '日期',
    time_id: '时段',
    capacity: '场地容量',
    place: '上课地点',
    teaches: '教授课程',
    username: '用户名',
    role_id: '角色',
    teacher_id: '关联教师'
};

function debounce(fn, delay) {
    let timeoutId = null;
    return function (...args) {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => {
            fn(...args);
        }, delay);
    };
}

export const useDataStore = defineStore('data', () => {
    const isInitialized = ref(false);
    const teachers = ref([]);
    const courses = ref([]);
    const campuses = ref([]);
    const venues = ref([]);
    const time = ref([]);
    const day = ref([]);
    const isReverting = ref(false);
    const isSolving = ref(false);
    const hasUnsavedChanges = ref(false);

    // Normalized relationship arrays
    const courseVenues = ref([]);
    const teacherCourses = ref([]);
    const scheduledClasses = ref([]);
    const teacherUnavailability = ref([]);
    const scheduleDensity = ref([]);

    // RBAC and Audit system arrays (Feature: 001-rbac-audit-system)
    const roles = ref([]);
    const users = ref([]);

    const resetState = () => {
        selectedCampusIdForCampusView.value = null;
        selectedTeacherIdForTeacherView.value = null;
        selectedVenueIdForCampusView.value = null;

        hasUnsavedChanges.value = false;
        isSolving.value = false;

        console.log("Data store state has been reset.");
    };

    const jsComputeDiff = (oldObj, newObj) => {
        const changes = [];
        const ignoreKeys = ['id', 'scheduled', 'unavailable', 'teacher_id'];
        const keys = new Set([...Object.keys(oldObj), ...Object.keys(newObj)]);

        const getName = (list, id) => list.find(item => item.id === id)?.name || id;
        const getValue = (list, id) => list.find(item => item.id === id)?.value || id;

        for (const key of keys) {
            if (ignoreKeys.includes(key)) continue;

            let oldVal = oldObj[key];
            let newVal = newObj[key];

            if (JSON.stringify(oldVal) === JSON.stringify(newVal)) continue;

            if (key === 'course_id') {
                oldVal = oldVal ? getName(courses.value, oldVal) : '无';
                newVal = newVal ? getName(courses.value, newVal) : '无';
            } else if (key === 'campus_id') {
                oldVal = oldVal ? getName(campuses.value, oldVal) : '无';
                newVal = newVal ? getName(campuses.value, newVal) : '无';
            } else if (key === 'venue_id') {
                oldVal = oldVal ? getName(venues.value, oldVal) : '无';
                newVal = newVal ? getName(venues.value, newVal) : '无';
            } else if (key === 'day_id') {
                oldVal = oldVal ? getValue(day.value, oldVal) : '无';
                newVal = newVal ? getValue(day.value, newVal) : '无';
            } else if (key === 'time_id') {
                oldVal = oldVal ? getValue(time.value, oldVal) : '无';
                newVal = newVal ? getValue(time.value, newVal) : '无';
            } else if (typeof oldVal === 'boolean' || typeof newVal === 'boolean') {
                oldVal = oldVal ? '是' : '否';
                newVal = newVal ? '是' : '否';
            }

            if (key === 'place' && (Array.isArray(oldVal) || Array.isArray(newVal))) {
                const transformPlace = (placeArr) => {
                    if (!Array.isArray(placeArr)) return '无';
                    return placeArr.map(p => {
                        const venue = venues.value.find(v => v.id === p.venue_id);
                        const campus = campuses.value.find(c => c.id === (p.campus_id || venue?.campus_id));
                        const vName = venue ? venue.name : (p.venue_id || '未知场地');
                        const cName = campus ? campus.name : '未知校区';
                        return `${cName} - ${vName}`;
                    }).join('; ');
                };
                oldVal = transformPlace(oldVal);
                newVal = transformPlace(newVal);
            }

            if (key === 'teaches' && (Array.isArray(oldVal) || Array.isArray(newVal))) {
                const transformTeaches = (ids) => {
                    if (!Array.isArray(ids)) return '无';
                    return ids.map(id => {
                        const c = courses.value.find(course => course.id === id);
                        return c ? c.name : '未知课程';
                    }).join('、');
                };
                oldVal = transformTeaches(oldVal);
                newVal = transformTeaches(newVal);
            }

            changes.push({
                field: key,
                label: FIELD_LABELS[key] || key,
                old: oldVal === undefined || oldVal === null ? '无' : oldVal,
                new: newVal === undefined || newVal === null ? '无' : newVal
            });
        }
        return changes;
    }

    const reportAudit = async (actionType, table, id, targetName, details = {}) => {
        const authStore = useAuthStore();
        if (!authStore.sessionId) return;

        try {
            const payload = {
                target_name: targetName,
                ...details
            };

            await invoke('record_audit_log', {
                actionType,
                targetTable: table,
                targetId: id,
                changeDetails: payload,
                sessionId: authStore.sessionId
            });
        } catch (err) {
            console.warn('audit log upload failed:', err);
        }
    };

    const selectedCampusIdForCampusView = ref(null);
    const selectedTeacherIdForTeacherView = ref(null);
    const selectedVenueIdForCampusView = ref(null);

    const syncUnsavedStatus = async () => {
        const status = await invoke('has_unsaved_changes');
        hasUnsavedChanges.value = status;
        console.log("Current Unsaved Status:", status);
    };

    const debouncedSave = debounce(async (data) => {
        console.log('Saving temp data to backend...');
        try {
            const rawData = {
                teachers: toRaw(data.teachers),
                courses: toRaw(data.courses),
                campuses: toRaw(data.campuses),
                venues: toRaw(data.venues),
                time: toRaw(data.time),
                day: toRaw(data.day),
                course_venues: toRaw(data.courseVenues),
                teacher_courses: toRaw(data.teacherCourses),
                scheduled_classes: toRaw(data.scheduledClasses),
                teacher_unavailability: toRaw(data.teacherUnavailability),
                schedule_density: toRaw(data.scheduleDensity),
            };
            await invoke('save_temp_data', { content: rawData });
            console.log('Temp data saved successfully.');
        } catch (error) {
            console.error('Failed to save temp data:', error);
        }
    }, 100);

    const initializeData = async () => {
        if (isInitialized.value) return;
        console.log('Initializing data from backend...');
        try {
            const loadedData = await invoke('load_data');
            replaceAllData(loadedData);
            isInitialized.value = true;
            await syncUnsavedStatus();

            console.log('Data initialized successfully.');

            // Set up event listener for commit-completed (T027)
            listen('commit-completed', () => {
                console.log('Commit completed event received');
                // Reset dirty state - no unsaved changes after commit
                // The watcher will handle any new changes automatically
            }).catch(err => {
                console.error('Failed to set up commit-completed listener:', err);
            });

            watch(
                () => ({
                    teachers: teachers.value,
                    courses: courses.value,
                    campuses: campuses.value,
                    venues: venues.value,
                    time: time.value,
                    day: day.value,
                    courseVenues: courseVenues.value,
                    teacherCourses: teacherCourses.value,
                    scheduledClasses: scheduledClasses.value,
                    teacherUnavailability: teacherUnavailability.value,
                    scheduleDensity: scheduleDensity.value
                }),
                (newState) => {
                    if (!isInitialized.value || isReverting.value) return;
                    hasUnsavedChanges.value = true;
                    debouncedSave(newState);
                },
                { deep: true }
            );

        } catch (error) {
            console.error('Failed to initialize data:', error);
        }
    };

    const replaceAllData = (newData) => {
        teachers.value = newData.teachers || [];
        courses.value = newData.courses || [];
        campuses.value = newData.campuses || [];
        venues.value = newData.venues || [];
        time.value = newData.time || [];
        day.value = newData.day || [];
        courseVenues.value = newData.course_venues || [];
        teacherCourses.value = newData.teacher_courses || [];
        scheduledClasses.value = newData.scheduled_classes || [];
        teacherUnavailability.value = newData.teacher_unavailability || [];
        scheduleDensity.value = newData.schedule_density || [];
        roles.value = newData.roles || [];
        users.value = newData.users || [];
        syncUnsavedStatus();
    };

    // Computed properties for normalized data relationships
    const venuesByCampus = computed(() => (campusId) => {
        return venues.value.filter(v => v.campus_id === campusId);
    });

    const courseVenuesByCourse = computed(() => (courseId) => {
        return courseVenues.value.filter(cv => cv.course_id === courseId);
    });

    const teacherCoursesByTeacher = computed(() => (teacherId) => {
        return teacherCourses.value.filter(tc => tc.teacher_id === teacherId);
    });

    const scheduledClassesByTeacher = computed(() => (teacherId) => {
        return scheduledClasses.value.filter(sc => sc.teacher_id === teacherId);
    });

    const teacherUnavailabilityByTeacher = computed(() => (teacherId) => {
        return teacherUnavailability.value.filter(tu => tu.teacher_id === teacherId);
    });

    const scheduleDensityByCampus = computed(() => (campusId) => {
        return scheduleDensity.value.filter(sd => sd.campus_id === campusId);
    });

    const revertChanges = async () => {
        console.log("Reverting changes...");
        isReverting.value = true;
        try {
            const authStore = useAuthStore();
            await invoke('clear_temp_data', {
                sessionId: authStore.sessionId
            });
            console.log("Temp data cleared.");
            const reloadedData = await invoke('load_data');
            console.log("Data reloaded from source.");
            replaceAllData(reloadedData);
            await syncUnsavedStatus();
            await emit('data-reloaded'); 
        } catch (error) {
            console.error("Failed to revert changes:", error);
            throw error;
        } finally {
            setTimeout(() => {
                isReverting.value = false;
            }, 0);
        }
    };

    const courseOptions = computed(() => courses.value.map(c => ({ label: c.name, value: c.id })));
    const campusOptions = computed(() => campuses.value.map(c => ({ label: c.name, value: c.id })));
    const venueOptionsByCampus = computed(() => (campusId) => {
        const campusVenues = venues.value.filter(v => v.campus_id === campusId);
        return campusVenues.map(v => ({ label: v.name, value: v.id }));
    });

    const addTeacher = (teacherData) => {
        const newId = uuidv4();
        const newTeacher = { ...teacherData, id: newId };
        teachers.value.push(newTeacher);

        // Handle teaches relationship
        if (teacherData.teaches && Array.isArray(teacherData.teaches)) {
            teacherData.teaches.forEach(courseId => {
                teacherCourses.value.push({ teacher_id: newTeacher.id, course_id: courseId });
            });
        }

        // Handle unavailable slots
        if (teacherData.unavailable && Array.isArray(teacherData.unavailable)) {
            teacherData.unavailable.forEach(slot => {
                teacherUnavailability.value.push({
                    teacher_id: newTeacher.id,
                    day_id: slot.day_id,
                    time_id: slot.time_id
                });
            });
        }

        // Handle scheduled classes
        if (teacherData.scheduled && Array.isArray(teacherData.scheduled)) {
            teacherData.scheduled.forEach(schedule => {
                scheduledClasses.value.push({
                    ...schedule,
                    id: schedule.id || uuidv4(),
                    teacher_id: newTeacher.id
                });
            });
        }
        reportAudit('TEACHER_CREATED', 'teachers', newId, teacherData.name, {
            action: '新增教师',
            max_teaching_hours: teacherData.max_teaching_hours,
            is_only_shahe: teacherData.is_only_shahe ? '是' : '否',
            teaches_count: teacherData.teaches?.length || 0
        });
    };

    const updateTeacher = (updatedTeacher) => {
        const index = teachers.value.findIndex(t => t.id === updatedTeacher.id);
        if (index !== -1) {
            const oldFlatTeacher = toRaw(teachers.value[index]);
            const oldCompositeTeacher = {
                ...oldFlatTeacher,
                teaches: teacherCourses.value
                    .filter(tc => tc.teacher_id === updatedTeacher.id)
                    .map(tc => tc.course_id)
            };

            const changes = jsComputeDiff(oldCompositeTeacher, updatedTeacher);

            if (changes.length > 0) {
                reportAudit('TEACHER_UPDATED', 'teachers', updatedTeacher.id, oldFlatTeacher.name, { changes });
            }
            const { teaches, ...basicInfo } = updatedTeacher;
            teachers.value[index] = { ...teachers.value[index], ...basicInfo };
            teacherCourses.value = teacherCourses.value.filter(
                tc => tc.teacher_id !== updatedTeacher.id
            );
            if (updatedTeacher.teaches && Array.isArray(updatedTeacher.teaches)) {
                updatedTeacher.teaches.forEach(courseId => {
                    teacherCourses.value.push({
                        teacher_id: updatedTeacher.id,
                        course_id: courseId
                    });
                });
            }
        }
    };

    const deleteTeacher = (teacherId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        const teacherName = teacher?.name || '未知教师';
        teachers.value = teachers.value.filter(t => t.id !== teacherId);
        // Remove related data
        teacherCourses.value = teacherCourses.value.filter(tc => tc.teacher_id !== teacherId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.teacher_id !== teacherId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.teacher_id !== teacherId);

        if (selectedTeacherIdForTeacherView.value === teacherId) {
            selectedTeacherIdForTeacherView.value = null;
        }
        reportAudit('TEACHER_DELETED', 'teachers', teacherId, teacherName, {
            action: '删除教师'
        });
    };

    const commitChanges = async () => {
        console.log('Committing changes to backend...');
        try {
            const authStore = useAuthStore();
            await invoke('commit_data', {
                sessionId: authStore.sessionId
            });
            console.log('Data committed successfully.');
            await syncUnsavedStatus();
        } catch (error) {
            console.error('Failed to commit data:', error);
            throw error;
        }
    };

    const addCourse = (courseData) => {
        const newId = uuidv4();
        const newCourse = { ...courseData, id: newId };
        courses.value.push(newCourse);

        // Handle place (venue) relationships
        if (courseData.place && Array.isArray(courseData.place)) {
            courseData.place.forEach(place => {
                courseVenues.value.push({
                    course_id: newCourse.id,
                    venue_id: place.venue_id
                });
            });
        }

        reportAudit('COURSE_CREATED', 'courses', newId, courseData.name, {
            action: '新增课程',
            venue_count: courseData.place?.length || 0
        });
    };

    const updateCourse = (updatedCourse) => {
        const index = courses.value.findIndex(c => c.id === updatedCourse.id);
        if (index !== -1) {
            const oldFlatCourse = toRaw(courses.value[index]);
            const oldCompositeCourse = {
                ...oldFlatCourse,
                place: courseVenues.value
                    .filter(cv => cv.course_id === updatedCourse.id)
                    .map(cv => ({ venue_id: cv.venue_id }))
            };

            const changes = jsComputeDiff(oldCompositeCourse, updatedCourse);

            if (changes.length > 0) {
                reportAudit('COURSE_UPDATED', 'courses', updatedCourse.id, oldFlatCourse.name, { changes });
            }

            const oldVenueIds = courseVenues.value
                .filter(cv => cv.course_id === updatedCourse.id)
                .map(cv => cv.venue_id);

            if (updatedCourse.place?.length === 1 && oldVenueIds.length === 1) {
                const newVenueId = updatedCourse.place[0].venue_id;
                const oldVenueId = oldVenueIds[0];

                if (newVenueId !== oldVenueId) {
                    scheduledClasses.value.forEach(sc => {
                        if (sc.course_id === updatedCourse.id && sc.venue_id === oldVenueId) {
                            sc.venue_id = newVenueId;
                            const v = venues.value.find(v => v.id === newVenueId);
                            if (v) sc.campus_id = v.campus_id;
                        }
                    });
                }
            }
            const { place, ...basicInfo } = updatedCourse;
            courses.value[index] = { ...courses.value[index], ...basicInfo };

            courseVenues.value = courseVenues.value.filter(cv => cv.course_id !== updatedCourse.id);

            if (updatedCourse.place && Array.isArray(updatedCourse.place)) {
                updatedCourse.place.forEach(p => {
                    if (p.venue_id) {
                        courseVenues.value.push({
                            course_id: updatedCourse.id,
                            venue_id: p.venue_id
                        });
                    }
                });
            }
        }
    };

    const deleteCourse = (courseId) => {
        const course = courses.value.find(c => c.id === courseId);
        const courseName = course?.name || '未知课程';
        courses.value = courses.value.filter(c => c.id !== courseId);
        courseVenues.value = courseVenues.value.filter(cv => cv.course_id !== courseId);
        teacherCourses.value = teacherCourses.value.filter(tc => tc.course_id !== courseId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.course_id !== courseId);
        reportAudit('COURSE_DELETED', 'courses', courseId, courseName, {
            action: '删除课程'
        });
    };

    const addCampus = (campusData) => {
        const newId = uuidv4();
        const newCampus = { ...campusData, id: newId };
        campuses.value.push(newCampus);

        reportAudit('CAMPUS_CREATED', 'campuses', newId, campusData.name);
        if (campusData.venues && Array.isArray(campusData.venues)) {
            campusData.venues.forEach(venue => {
                venues.value.push({
                    ...venue,
                    id: venue.id || uuidv4(),
                    campus_id: newCampus.id
                });
            });
        }

        if (campusData.schedule_density && Array.isArray(campusData.schedule_density)) {
            campusData.schedule_density.forEach(density => {
                scheduleDensity.value.push({
                    campus_id: newCampus.id,
                    day_id: density.day_id,
                    time_id: density.time_id,
                    count: density.count
                });
            });
        }
    };
    const updateCampus = (updatedCampus) => {
        const index = campuses.value.findIndex(c => c.id === updatedCampus.id);
        if (index !== -1) {
            const oldCampus = toRaw(campuses.value[index]);
            const changes = jsComputeDiff(oldCampus, updatedCampus);
            if (changes.length > 0) {
                reportAudit('CAMPUS_UPDATED', 'campuses', updatedCampus.id, oldCampus.name, { changes });
            }
            campuses.value[index] = { ...campuses.value[index], ...updatedCampus };
        }
    };
    const deleteCampus = (campusId) => {
        const campus = campuses.value.find(c => c.id === campusId);
        const campusName = campus?.name || '未知校区';
        campuses.value = campuses.value.filter(c => c.id !== campusId);

        const venueIds = venues.value.filter(v => v.campus_id === campusId).map(v => v.id);

        venues.value = venues.value.filter(v => v.campus_id !== campusId);

        courseVenues.value = courseVenues.value.filter(cv => !venueIds.includes(cv.venue_id));

        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.campus_id !== campusId);

        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.campus_id !== campusId);

        if (selectedCampusIdForCampusView.value === campusId) {
            selectedCampusIdForCampusView.value = null;
        }

        reportAudit('CAMPUS_DELETED', 'campuses', campusId, campusName);
    };

    const addVenueToCampus = (campusId, venueData) => {
        const newId = uuidv4();
        const campus = campuses.value.find(c => c.id === campusId);
        const newVenue = { ...venueData, id: newId, campus_id: campusId };
        venues.value.push(newVenue);

        reportAudit('VENUE_CREATED', 'venues', newId, `${campus?.name} - ${venueData.name}`);
    };

    const updateVenueInCampus = (campusId, updatedVenue) => {
        const index = venues.value.findIndex(v => v.id === updatedVenue.id);
        if (index !== -1) {
            const oldVenue = toRaw(venues.value[index]);
            const campus = campuses.value.find(c => c.id === campusId);
            const changes = jsComputeDiff(oldVenue, updatedVenue);

            if (changes.length > 0) {
                reportAudit('VENUE_UPDATED', 'venues', updatedVenue.id, `${campus?.name} - ${oldVenue.name}`, { changes });
            }
            venues.value[index] = { ...venues.value[index], ...updatedVenue, campus_id: campusId };
        }
    };

    const deleteVenueFromCampus = (campusId, venueId) => {
        const venue = venues.value.find(v => v.id === venueId);
        const campus = campuses.value.find(c => c.id === campusId);
        venues.value = venues.value.filter(v => !(v.id === venueId && v.campus_id === campusId));
        courseVenues.value = courseVenues.value.filter(cv => cv.venue_id !== venueId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => !(sc.campus_id === campusId && sc.venue_id === venueId));
        reportAudit('VENUE_DELETED', 'venues', venueId, `${campus?.name} - ${venue?.name}`);
    };

    const updateCampusScheduleDensity = (campusId, dayId, timeId, count) => {
        const campus = campuses.value.find(c => c.id === campusId);
        const dayObj = day.value.find(d => d.id === dayId);
        const timeObj = time.value.find(t => t.id === timeId);
        const targetDesc = `${campus?.name || '未知校区'} (${dayObj?.value || '未知日期'} ${timeObj?.value || '未知时段'})`;
        const densityIndex = scheduleDensity.value.findIndex(
            d => d.campus_id === campusId && d.day_id === dayId && d.time_id === timeId
        );

        const oldCount = densityIndex !== -1 ? scheduleDensity.value[densityIndex].count : 0;
        const newCount = Math.max(0, count || 0);

        if (densityIndex !== -1) {
            if (newCount === 0) {
                scheduleDensity.value.splice(densityIndex, 1);
            } else {
                scheduleDensity.value[densityIndex].count = newCount;
            }
        } else if (newCount > 0) {
            scheduleDensity.value.push({
                campus_id: campusId,
                day_id: dayId,
                time_id: timeId,
                count: newCount
            });
        }
        if (oldCount !== newCount) {
            reportAudit('CAMPUS_UPDATED', 'schedule_density', campusId, targetDesc, {
                action: '修改期望排课密度',
                changes: [
                    {
                        field: 'count',
                        label: '期望班级数',
                        old: oldCount,
                        new: newCount
                    }
                ]
            });
        }
    };

    const getExpectedCountForCampusCell = computed(() => (campusId, dayId, timeId) => {
        const density = scheduleDensity.value.find(
            d => d.campus_id === campusId && d.day_id === dayId && d.time_id === timeId
        );
        return density ? density.count : 0;
    });

    const teacherOptions = computed(() => teachers.value.map(t => ({ label: t.name, value: t.id })));

    const getScheduleMapForTeacher = computed(() => (teacherId) => {
        const schedules = scheduledClasses.value.filter(sc => sc.teacher_id === teacherId);
        const scheduleMap = new Map();
        schedules.forEach(s => {
            const key = `${s.day_id}-${s.time_id}`;
            scheduleMap.set(key, s);
        });
        return scheduleMap;
    });

    const getUnavailableMapForTeacher = computed(() => (teacherId) => {
        const unavailableSlots = teacherUnavailability.value.filter(tu => tu.teacher_id === teacherId);
        const unavailableSet = new Set();
        unavailableSlots.forEach(slot => {
            const key = `${slot.day_id}-${slot.time_id}`;
            unavailableSet.add(key);
        });
        return unavailableSet;
    });

    const getScheduledClassesByCampus = computed(() => {
        const campusId = selectedCampusIdForCampusView.value;
        const venueId = selectedVenueIdForCampusView.value;

        if (!campusId) return new Map();

        const scheduleMap = new Map();
        const filteredSchedules = scheduledClasses.value.filter(schedule => {
            const campusMatch = schedule.campus_id === campusId;
            const venueMatch = !venueId || schedule.venue_id === venueId;
            return campusMatch && venueMatch;
        });

        filteredSchedules.forEach(schedule => {
            const teacher = teachers.value.find(t => t.id === schedule.teacher_id);
            if (teacher) {
                const key = `${schedule.day_id}-${schedule.time_id}`;
                if (!scheduleMap.has(key)) {
                    scheduleMap.set(key, []);
                }
                scheduleMap.get(key).push({ schedule, teacher });
            }
        });

        return scheduleMap;
    });

    const teacherCourseOptions = computed(() => (teacherId) => {
        const teacherCourseRelations = teacherCourses.value.filter(tc => tc.teacher_id === teacherId);
        const courseIds = teacherCourseRelations.map(tc => tc.course_id);
        return courses.value
            .filter(course => courseIds.includes(course.id))
            .map(course => ({ label: course.name, value: course.id }));
    });

    const courseCampusOptions = computed(() => (courseId) => {
        const courseVenueRelations = courseVenues.value.filter(cv => cv.course_id === courseId);
        const venueIds = courseVenueRelations.map(cv => cv.venue_id);
        const courseVenuesList = venues.value.filter(v => venueIds.includes(v.id));
        const campusIds = new Set(courseVenuesList.map(v => v.campus_id));
        return campuses.value
            .filter(campus => campusIds.has(campus.id))
            .map(campus => ({ label: campus.name, value: campus.id }));
    });

    const courseVenueOptions = computed(() => (courseId, campusId) => {
        const courseVenueRelations = courseVenues.value.filter(cv => cv.course_id === courseId);
        const venueIds = new Set(courseVenueRelations.map(cv => cv.venue_id));
        const campusVenues = venues.value.filter(v => v.campus_id === campusId);
        return campusVenues
            .filter(venue => venueIds.has(venue.id))
            .map(venue => ({ label: venue.name, value: venue.id }));
    });

    const addSchedule = (teacherId, scheduleData) => {
        const newId = uuidv4();
        const newSchedule = { ...scheduleData, id: newId, teacher_id: teacherId };
        scheduledClasses.value.push(newSchedule);

        const teacher = teachers.value.find(t => t.id === teacherId);
        const course = courses.value.find(c => c.id === scheduleData.course_id);
        const targetName = `${teacher?.name || '未知教师'} - ${course?.name || '未知课程'}`;

        const dayObj = day.value.find(d => d.id === scheduleData.day_id);
        const timeObj = time.value.find(t => t.id === scheduleData.time_id);
        const campusObj = campuses.value.find(c => c.id === scheduleData.campus_id);
        const venueObj = venues.value.find(v => v.id === scheduleData.venue_id);

        reportAudit('SCHEDULE_MODIFIED', 'scheduled_classes', newId, targetName, {
            action: '手工新增',
            teacher_name: teacher?.name,
            course_name: course?.name,
            time_slot_name: `${dayObj?.value || '未知日期'} ${timeObj?.value || '未知时段'}`,
            location_name: `${campusObj?.name || '未知校区'} - ${venueObj?.name || '未知场地'}`
        });
    };

    const updateSchedule = (teacherId, updatedSchedule) => {
        const index = scheduledClasses.value.findIndex(s => s.id === updatedSchedule.id);
        if (index !== -1) {
            const oldSchedule = toRaw(scheduledClasses.value[index]);
            const teacher = teachers.value.find(t => t.id === teacherId);
            const course = courses.value.find(c => c.id === oldSchedule.course_id);

            const changes = jsComputeDiff(oldSchedule, updatedSchedule);
            if (changes.length > 0) {
                reportAudit('SCHEDULE_MODIFIED', 'scheduled_classes', updatedSchedule.id,
                    `${teacher?.name} - ${course?.name}`,
                    { action: '手工修改', changes }
                );
            }
            scheduledClasses.value[index] = { ...scheduledClasses.value[index], ...updatedSchedule, teacher_id: teacherId };
        }
    };

    const deleteSchedule = (scheduleId) => {
        const schedule = scheduledClasses.value.find(s => s.id === scheduleId);

        const teacher = teachers.value.find(t => t.id === schedule?.teacher_id);
        const course = courses.value.find(c => c.id === schedule?.course_id);
        const targetName = `${teacher?.name || '未知教师'} - ${course?.name || '未知课程'}`;

        const dayObj = day.value.find(d => d.id === schedule?.day_id);
        const timeObj = time.value.find(t => t.id === schedule?.time_id);
        const campusObj = campuses.value.find(c => c.id === schedule?.campus_id);
        const venueObj = venues.value.find(v => v.id === schedule?.venue_id);

        scheduledClasses.value = scheduledClasses.value.filter(s => s.id !== scheduleId);

        scheduledClasses.value = scheduledClasses.value.filter(s => s.id !== scheduleId);

        reportAudit('SCHEDULE_MODIFIED', 'scheduled_classes', scheduleId, targetName, {
            action: '手工删除',
            summary: `删除了 ${dayObj?.value} ${timeObj?.value} 在 ${campusObj?.name} ${venueObj?.name} 的课程`
        });
    };

    const toggleUnavailableSlot = (teacherId, dayId, timeId) => {
        const index = teacherUnavailability.value.findIndex(
            tu => tu.teacher_id === teacherId && tu.day_id === dayId && tu.time_id === timeId
        );

        if (index !== -1) {
            teacherUnavailability.value.splice(index, 1);
        } else {
            teacherUnavailability.value.push({ teacher_id: teacherId, day_id: dayId, time_id: timeId });
        }

        const teacher = teachers.value.find(t => t.id === teacherId);
        reportAudit('TEACHER_UPDATED', 'teachers', teacherId, teacher?.name, {
            action: '修改不可用时段',
            details: { dayId, timeId }
        });
    };

    const addTimeSlot = (timeSlotData) => {
        const newId = uuidv4();
        time.value.push({ ...timeSlotData, id: newId });
        reportAudit('TIME_SLOT_MODIFIED', 'time_slots', newId, '时间段配置', {
            action: '新增时间段',
            value: timeSlotData.value,
            corresponding_hours: timeSlotData.corresponding_hours
        });
    };
    const updateTimeSlot = (updatedTimeSlot) => {
        const index = time.value.findIndex(t => t.id === updatedTimeSlot.id);
        if (index !== -1) {
            const old = toRaw(time.value[index]);
            const changes = jsComputeDiff(old, updatedTimeSlot);
            if (changes.length > 0) {
                reportAudit('TIME_SLOT_MODIFIED', 'time_slots', updatedTimeSlot.id, '时间段配置', {
                    action: `修改: ${old.value}`,
                    changes
                });
            }
            time.value[index] = { ...time.value[index], ...updatedTimeSlot };
        }
    };
    const deleteTimeSlot = (timeSlotId) => {
        const slot = time.value.find(t => t.id === timeSlotId);
        const slotName = slot?.value || '未知';
        time.value = time.value.filter(t => t.id !== timeSlotId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.time_id !== timeSlotId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.time_id !== timeSlotId);
        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.time_id !== timeSlotId);
        reportAudit('TIME_SLOT_MODIFIED', 'time_slots', timeSlotId, '时间段配置', {
            action: '删除时间段',
            name: slotName
        });
    };

    const addDay = (dayData) => {
        const newId = uuidv4();
        day.value.push({ ...dayData, id: newId });
        reportAudit('DAY_MODIFIED', 'days', newId, '工作日配置', {
            action: '新增工作日',
            value: dayData.value
        });
    };
    const updateDay = (updatedDay) => {
        const index = day.value.findIndex(d => d.id === updatedDay.id);
        if (index !== -1) {
            const old = toRaw(day.value[index]);
            if (old.value !== updatedDay.value) {
                const changes = [{
                    field: 'value',
                    label: '名称',
                    old: old.value,
                    new: updatedDay.value
                }];
                reportAudit('DAY_MODIFIED', 'days', updatedDay.id, '工作日配置', {
                    action: `修改: ${old.value}`,
                    changes
                });
            }
            day.value[index] = { ...day.value[index], ...updatedDay };
        }
    };
    const deleteDay = (dayId) => {
        const d = day.value.find(item => item.id === dayId);
        const dayName = d?.value || '未知';
        day.value = day.value.filter(d => d.id !== dayId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.day_id !== dayId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.day_id !== dayId);
        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.day_id !== dayId);
        reportAudit('DAY_MODIFIED', 'days', dayId, '工作日配置', {
            action: '删除工作日',
            name: dayName
        });
    };

    return {
        isInitialized,
        isSolving,
        teachers,
        courses,
        campuses,
        venues,
        time,
        day,
        courseVenues,
        teacherCourses,
        scheduledClasses,
        teacherUnavailability,
        scheduleDensity,
        roles,
        users,
        initializeData,
        replaceAllData,
        revertChanges,
        commitChanges,
        hasUnsavedChanges,
        syncUnsavedStatus,
        resetState,

        getUnavailableMapForTeacher,
        toggleUnavailableSlot,

        selectedCampusIdForCampusView,
        selectedTeacherIdForTeacherView,
        selectedVenueIdForCampusView,

        courseOptions,
        campusOptions,
        venueOptionsByCampus,
        venuesByCampus,
        courseVenuesByCourse,
        teacherCoursesByTeacher,
        scheduledClassesByTeacher,
        teacherUnavailabilityByTeacher,
        scheduleDensityByCampus,

        addTeacher,
        updateTeacher,
        deleteTeacher,

        addCourse,
        updateCourse,
        deleteCourse,

        addCampus,
        updateCampus,
        deleteCampus,

        addVenueToCampus,
        updateVenueInCampus,
        deleteVenueFromCampus,

        teacherOptions,
        getScheduleMapForTeacher,
        getScheduledClassesByCampus,
        teacherCourseOptions,
        courseCampusOptions,
        courseVenueOptions,
        addSchedule,
        updateSchedule,
        deleteSchedule,

        addTimeSlot,
        updateTimeSlot,
        deleteTimeSlot,
        addDay,
        updateDay,
        deleteDay,
        getExpectedCountForCampusCell,
        updateCampusScheduleDensity,
    };
});