import { defineStore } from 'pinia';
import { ref, watch, toRaw, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { v4 as uuidv4 } from 'uuid';
import { useAuthStore } from './auth';

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

    const reportAudit = async (actionType, table, id, details) => {
        const authStore = useAuthStore();
        if (!authStore.sessionId) return;

        try {
            await invoke('record_audit_log', {
                actionType,
                targetTable: table,
                targetId: id,
                changeDetails: details,
                sessionId: authStore.sessionId
            });
        } catch (err) {
            console.warn('audit upload failed', err);
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
        reportAudit('TEACHER_CREATED', 'teachers', newId, { name: teacherData.name });
    };
    const updateTeacher = (updatedTeacher) => {
        const index = teachers.value.findIndex(t => t.id === updatedTeacher.id);
        if (index !== -1) {
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

            console.log('教师关联课程已同步更新');
        }
    };
    const deleteTeacher = (teacherId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        teachers.value = teachers.value.filter(t => t.id !== teacherId);
        // Remove related data
        teacherCourses.value = teacherCourses.value.filter(tc => tc.teacher_id !== teacherId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.teacher_id !== teacherId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.teacher_id !== teacherId);

        if (selectedTeacherIdForTeacherView.value === teacherId) {
            selectedTeacherIdForTeacherView.value = null;
        }
        reportAudit('TEACHER_DELETED', 'teachers', teacherId, { name: teacher?.name });
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
        const newCourse = { ...data, id: newId };
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

        reportAudit('COURSE_CREATED', 'courses', newId, { name: courseData.name });
    };

    const updateCourse = (updatedCourse) => {
        const index = courses.value.findIndex(c => c.id === updatedCourse.id);
        if (index !== -1) {
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
                            // 同时更新校区 ID
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
            console.log('课程关联场地已同步更新');
        }
    };

    const deleteCourse = (courseId) => {
        courses.value = courses.value.filter(c => c.id !== courseId);
        courseVenues.value = courseVenues.value.filter(cv => cv.course_id !== courseId);
        teacherCourses.value = teacherCourses.value.filter(tc => tc.course_id !== courseId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.course_id !== courseId);
    };

    const addCampus = (campusData) => {
        const { id, ...data } = campusData;
        const newCampus = { ...data, id: uuidv4() };
        campuses.value.push(newCampus);

        // Handle venues
        if (campusData.venues && Array.isArray(campusData.venues)) {
            campusData.venues.forEach(venue => {
                venues.value.push({
                    ...venue,
                    id: venue.id || uuidv4(),
                    campus_id: newCampus.id
                });
            });
        }

        // Handle schedule_density
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
            campuses.value[index] = { ...campuses.value[index], ...updatedCampus };
        }
    };
    const deleteCampus = (campusId) => {
        campuses.value = campuses.value.filter(c => c.id !== campusId);

        // Get venue IDs for this campus
        const venueIds = venues.value.filter(v => v.campus_id === campusId).map(v => v.id);

        // Remove venues for this campus
        venues.value = venues.value.filter(v => v.campus_id !== campusId);

        // Remove course_venues for venues in this campus
        courseVenues.value = courseVenues.value.filter(cv => !venueIds.includes(cv.venue_id));

        // Remove scheduled classes for this campus
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.campus_id !== campusId);

        // Remove schedule density for this campus
        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.campus_id !== campusId);

        if (selectedCampusIdForCampusView.value === campusId) {
            selectedCampusIdForCampusView.value = null;
        }
    };

    const addVenueToCampus = (campusId, venueData) => {
        const { id, ...data } = venueData;
        const newVenue = { ...data, id: uuidv4(), campus_id: campusId };
        venues.value.push(newVenue);
    };

    const updateVenueInCampus = (campusId, updatedVenue) => {
        const venueIndex = venues.value.findIndex(v => v.id === updatedVenue.id && v.campus_id === campusId);
        if (venueIndex !== -1) {
            venues.value[venueIndex] = { ...venues.value[venueIndex], ...updatedVenue, campus_id: campusId };
        }
    };

    const deleteVenueFromCampus = (campusId, venueId) => {
        venues.value = venues.value.filter(v => !(v.id === venueId && v.campus_id === campusId));
        courseVenues.value = courseVenues.value.filter(cv => cv.venue_id !== venueId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => !(sc.campus_id === campusId && sc.venue_id === venueId));
    };

    const updateCampusScheduleDensity = (campusId, dayId, timeId, count) => {
        const densityIndex = scheduleDensity.value.findIndex(
            d => d.campus_id === campusId && d.day_id === dayId && d.time_id === timeId
        );

        const newCount = Math.max(0, count || 0);

        if (densityIndex !== -1) {
            if (newCount === 0) {
                scheduleDensity.value.splice(densityIndex, 1);
            } else {
                scheduleDensity.value[densityIndex].count = newCount;
            }
        } else if (newCount > 0) {
            scheduleDensity.value.push({ campus_id: campusId, day_id: dayId, time_id: timeId, count: newCount });
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
        reportAudit('SCHEDULE_MODIFIED', 'scheduled_classes', newId, {
            action: '手工新增',
            teacher_id: teacherId,
            course_id: scheduleData.course_id,
            time_slot: `${scheduleData.day_id}-${scheduleData.time_id}`
        });
    };

    const updateSchedule = (teacherId, updatedSchedule) => {
        const index = scheduledClasses.value.findIndex(s => s.id === updatedSchedule.id && s.teacher_id === teacherId);
        if (index !== -1) {
            scheduledClasses.value[index] = { ...scheduledClasses.value[index], ...updatedSchedule, teacher_id: teacherId };
        }
    };

    const deleteSchedule = (teacherId, scheduleId) => {
        const schedule = scheduledClasses.value.find(s => s.id === scheduleId);
        scheduledClasses.value = scheduledClasses.value.filter(s => s.id !== scheduleId);

        reportAudit('SCHEDULE_MODIFIED', 'scheduled_classes', scheduleId, {
            action: '手工删除',
            old_data: schedule
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
    };

    const addTimeSlot = (timeSlotData) => {
        const { id, ...data } = timeSlotData;
        const newTimeSlot = { ...data, id: uuidv4() };
        time.value.push(newTimeSlot);
    };
    const updateTimeSlot = (updatedTimeSlot) => {
        const index = time.value.findIndex(t => t.id === updatedTimeSlot.id);
        if (index !== -1) {
            time.value[index] = { ...time.value[index], ...updatedTimeSlot };
        }
    };
    const deleteTimeSlot = (timeSlotId) => {
        time.value = time.value.filter(t => t.id !== timeSlotId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.time_id !== timeSlotId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.time_id !== timeSlotId);
        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.time_id !== timeSlotId);
    };

    const addDay = (dayData) => {
        const { id, ...data } = dayData;
        const newDay = { ...data, id: uuidv4() };
        day.value.push(newDay);
    };
    const updateDay = (updatedDay) => {
        const index = day.value.findIndex(d => d.id === updatedDay.id);
        if (index !== -1) {
            day.value[index] = { ...day.value[index], ...updatedDay };
        }
    };
    const deleteDay = (dayId) => {
        day.value = day.value.filter(d => d.id !== dayId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.day_id !== dayId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.day_id !== dayId);
        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.day_id !== dayId);
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