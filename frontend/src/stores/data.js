import { defineStore } from 'pinia';
import { ref, watch, toRaw, computed } from 'vue';
import { invoke, listen, emit } from '../host/desktop';
import { v4 as uuidv4 } from 'uuid';

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

    const courseVenues = ref([]);
    const teacherCourses = ref([]);
    const teacherCampuses = ref([]);
    const scheduledClasses = ref([]);
    const teacherUnavailability = ref([]);
    const scheduleDensity = ref([]);

    const resetState = () => {
        selectedCampusIdForCampusView.value = null;
        selectedTeacherIdForTeacherView.value = null;
        selectedVenueIdForCampusView.value = null;

        hasUnsavedChanges.value = false;
        isSolving.value = false;

        console.log("Data store state has been reset.");
    };

    const resolveTeacherCampusIds = (campusIds) => {
        const validCampusIds = campuses.value.map(campus => campus.id);
        const selectedIds = Array.isArray(campusIds)
            ? [...new Set(campusIds.filter(id => validCampusIds.includes(id)))]
            : [];

        return selectedIds.length > 0 ? selectedIds : [...validCampusIds];
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
                teacher_campuses: toRaw(data.teacherCampuses),
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

            listen('commit-completed', () => {
                console.log('Commit completed event received');
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
                    teacherCampuses: teacherCampuses.value,
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
        teacherCampuses.value = newData.teacher_campuses || [];
        scheduledClasses.value = newData.scheduled_classes || [];
        teacherUnavailability.value = newData.teacher_unavailability || [];
        scheduleDensity.value = newData.schedule_density || [];
        syncUnsavedStatus();
    };

    const venuesByCampus = computed(() => (campusId) => {
        return venues.value.filter(v => v.campus_id === campusId);
    });

    const courseVenuesByCourse = computed(() => (courseId) => {
        return courseVenues.value.filter(cv => cv.course_id === courseId);
    });

    const teacherCoursesByTeacher = computed(() => (teacherId) => {
        return teacherCourses.value.filter(tc => tc.teacher_id === teacherId);
    });

    const teacherCampusesByTeacher = computed(() => (teacherId) => {
        return teacherCampuses.value.filter(tc => tc.teacher_id === teacherId);
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
            await invoke('clear_temp_data');
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
        const { teaches, campus_ids, ...teacherFields } = teacherData;
        const newTeacher = { ...teacherFields, id: newId };
        teachers.value.push(newTeacher);

        if (teaches && Array.isArray(teaches)) {
            teaches.forEach(courseId => {
                teacherCourses.value.push({ teacher_id: newTeacher.id, course_id: courseId });
            });
        }

        resolveTeacherCampusIds(campus_ids).forEach(campusId => {
            teacherCampuses.value.push({ teacher_id: newTeacher.id, campus_id: campusId });
        });

        if (teacherData.unavailable && Array.isArray(teacherData.unavailable)) {
            teacherData.unavailable.forEach(slot => {
                teacherUnavailability.value.push({
                    teacher_id: newTeacher.id,
                    day_id: slot.day_id,
                    time_id: slot.time_id
                });
            });
        }

        if (teacherData.scheduled && Array.isArray(teacherData.scheduled)) {
            teacherData.scheduled.forEach(schedule => {
                scheduledClasses.value.push({
                    ...schedule,
                    id: schedule.id || uuidv4(),
                    teacher_id: newTeacher.id
                });
            });
        }
    };

    const updateTeacher = (updatedTeacher) => {
        const index = teachers.value.findIndex(t => t.id === updatedTeacher.id);
        if (index !== -1) {
            const { teaches, campus_ids, ...basicInfo } = updatedTeacher;
            teachers.value[index] = { ...teachers.value[index], ...basicInfo };
            teacherCourses.value = teacherCourses.value.filter(
                tc => tc.teacher_id !== updatedTeacher.id
            );
            teacherCampuses.value = teacherCampuses.value.filter(
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
            resolveTeacherCampusIds(campus_ids).forEach(campusId => {
                teacherCampuses.value.push({
                    teacher_id: updatedTeacher.id,
                    campus_id: campusId
                });
            });
        }
    };

    const deleteTeacher = (teacherId) => {
        teachers.value = teachers.value.filter(t => t.id !== teacherId);
        teacherCourses.value = teacherCourses.value.filter(tc => tc.teacher_id !== teacherId);
        teacherCampuses.value = teacherCampuses.value.filter(tc => tc.teacher_id !== teacherId);
        teacherUnavailability.value = teacherUnavailability.value.filter(tu => tu.teacher_id !== teacherId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.teacher_id !== teacherId);

        if (selectedTeacherIdForTeacherView.value === teacherId) {
            selectedTeacherIdForTeacherView.value = null;
        }
    };

    const commitChanges = async () => {
        console.log('Committing changes to backend...');
        try {
            await invoke('commit_data');
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

        if (courseData.place && Array.isArray(courseData.place)) {
            courseData.place.forEach(place => {
                courseVenues.value.push({
                    course_id: newCourse.id,
                    venue_id: place.venue_id
                });
            });
        }

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
        courses.value = courses.value.filter(c => c.id !== courseId);
        courseVenues.value = courseVenues.value.filter(cv => cv.course_id !== courseId);
        teacherCourses.value = teacherCourses.value.filter(tc => tc.course_id !== courseId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.course_id !== courseId);
    };

    const addCampus = (campusData) => {
        const newId = uuidv4();
        const newCampus = { ...campusData, id: newId };
        campuses.value.push(newCampus);

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
            campuses.value[index] = { ...campuses.value[index], ...updatedCampus };
        }
    };
    const deleteCampus = (campusId) => {
        campuses.value = campuses.value.filter(c => c.id !== campusId);

        const venueIds = venues.value.filter(v => v.campus_id === campusId).map(v => v.id);

        venues.value = venues.value.filter(v => v.campus_id !== campusId);

        courseVenues.value = courseVenues.value.filter(cv => !venueIds.includes(cv.venue_id));
        teacherCampuses.value = teacherCampuses.value.filter(tc => tc.campus_id !== campusId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => sc.campus_id !== campusId);
        scheduleDensity.value = scheduleDensity.value.filter(sd => sd.campus_id !== campusId);

        if (selectedCampusIdForCampusView.value === campusId) {
            selectedCampusIdForCampusView.value = null;
        }
    };

    const addVenueToCampus = (campusId, venueData) => {
        const newId = uuidv4();
        const newVenue = { ...venueData, id: newId, campus_id: campusId };
        venues.value.push(newVenue);
    };

    const updateVenueInCampus = (campusId, updatedVenue) => {
        const index = venues.value.findIndex(v => v.id === updatedVenue.id);
        if (index !== -1) {
            venues.value[index] = { ...venues.value[index], ...updatedVenue, campus_id: campusId };
        }
    };

    const deleteVenueFromCampus = (campusId, venueId) => {
        venues.value = venues.value.filter(v => !(v.id === venueId && v.campus_id === campusId));
        courseVenues.value = courseVenues.value.filter(cv => cv.venue_id !== venueId);
        scheduledClasses.value = scheduledClasses.value.filter(sc => !(sc.campus_id === campusId && sc.venue_id === venueId));
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

    const courseCampusOptions = computed(() => (courseId, teacherId = null) => {
        const courseVenueRelations = courseVenues.value.filter(cv => cv.course_id === courseId);
        const venueIds = courseVenueRelations.map(cv => cv.venue_id);
        const courseVenuesList = venues.value.filter(v => venueIds.includes(v.id));
        const courseCampusIds = new Set(courseVenuesList.map(v => v.campus_id));
        const teacherCampusRelations = teacherId ? teacherCampusesByTeacher.value(teacherId) : [];
        const allowedCampusIds = teacherId
            ? new Set((teacherCampusRelations.length > 0 ? teacherCampusRelations : campuses.value.map(campus => ({ campus_id: campus.id }))).map(rel => rel.campus_id))
            : null;

        return campuses.value
            .filter(campus => courseCampusIds.has(campus.id) && (!allowedCampusIds || allowedCampusIds.has(campus.id)))
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
    };

    const updateSchedule = (teacherId, updatedSchedule) => {
        const index = scheduledClasses.value.findIndex(s => s.id === updatedSchedule.id);
        if (index !== -1) {
            scheduledClasses.value[index] = { ...scheduledClasses.value[index], ...updatedSchedule, teacher_id: teacherId };
        }
    };

    const deleteSchedule = (scheduleId) => {
        scheduledClasses.value = scheduledClasses.value.filter(s => s.id !== scheduleId);
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
        const newId = uuidv4();
        time.value.push({ ...timeSlotData, id: newId });
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
        const newId = uuidv4();
        day.value.push({ ...dayData, id: newId });
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
        teacherCampuses,
        scheduledClasses,
        teacherUnavailability,
        scheduleDensity,
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
        teacherCampusesByTeacher,
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