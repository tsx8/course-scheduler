import { defineStore } from 'pinia';
import { ref, watch, toRaw, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
    const time = ref([]);
    const isReverting = ref(false);
    const day = ref([]);
    const isSolving = ref(false);

    const selectedCampusIdForCampusView = ref(null);
    const selectedTeacherIdForTeacherView = ref(null);
    const selectedVenueIdForCampusView = ref(null); 

    const debouncedSave = debounce(async (data) => {
        console.log('Saving temp data to backend...');
        try {
            const rawData = {
                teachers: toRaw(data.teachers),
                courses: toRaw(data.courses),
                campuses: toRaw(data.campuses),
                time: toRaw(data.time),
                day: toRaw(data.day),
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
            console.log('Raw loaded data:', loadedData);

            replaceAllData(loadedData);

            isInitialized.value = true;
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
                    time: time.value,
                    day: day.value
                }),
                (newState) => {
                    if (!isInitialized.value || isReverting.value) return;
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
        campuses.value = (newData.campuses || []).map(c => ({
            ...c,
            schedule_density: c.schedule_density || []
        }));
        time.value = newData.time || [];
        day.value = newData.day || [];
    };

    const revertChanges = async () => {
        console.log("Reverting changes...");
        isReverting.value = true;
        try {
            await invoke('clear_temp_data');
            console.log("Temp data cleared.");
            const reloadedData = await invoke('load_data');
            console.log("Data reloaded from source.");
            replaceAllData(reloadedData);
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
        const campus = campuses.value.find(c => c.id === campusId);
        return campus ? campus.venues.map(v => ({ label: v.name, value: v.id })) : [];
    });

    const addTeacher = (teacherData) => {
        const { id, ...data } = teacherData;
        const newTeacher = { ...data, id: uuidv4(), scheduled: teacherData.scheduled || [], unavailable: teacherData.unavailable || [] };
        teachers.value.push(newTeacher);
    };
    const updateTeacher = (updatedTeacher) => {
        const index = teachers.value.findIndex(t => t.id === updatedTeacher.id);
        if (index !== -1) {
            teachers.value[index] = { ...teachers.value[index], ...updatedTeacher };
        }
    };
    const deleteTeacher = (teacherId) => {
        teachers.value = teachers.value.filter(t => t.id !== teacherId);
        if (selectedTeacherIdForTeacherView.value === teacherId) {
            selectedTeacherIdForTeacherView.value = null;
        }
    };

    const commitChanges = async () => {
        console.log('Committing changes to backend...');
        try {
            await invoke('commit_data');
            console.log('Data committed successfully.');
        } catch (error) {
            console.error('Failed to commit data:', error);
            throw error;
        }
    };

    const addCourse = (courseData) => {
        const { id, ...data } = courseData;
        const newCourse = { ...data, id: uuidv4(), place: courseData.place || [] };
        courses.value.push(newCourse);
    };
    
    const updateCourse = (updatedCourse) => {
        const index = courses.value.findIndex(c => c.id === updatedCourse.id);
        if (index !== -1) {
            const originalCourse = JSON.parse(JSON.stringify(courses.value[index]));

            const placeKey = p => `${p.campus_id}-${p.venue_id}`;

            const originalPlaces = new Set(originalCourse.place.map(placeKey));
            const updatedPlaces = new Set(updatedCourse.place.map(placeKey));

            const removedPlacesRaw = originalCourse.place.filter(p => !updatedPlaces.has(placeKey(p)));
            const addedPlacesRaw = updatedCourse.place.filter(p => !originalPlaces.has(placeKey(p)));

            const campusToRemovedVenue = new Map();
            removedPlacesRaw.forEach(p => {
                if (!campusToRemovedVenue.has(p.campus_id)) {
                    campusToRemovedVenue.set(p.campus_id, []);
                }
                campusToRemovedVenue.get(p.campus_id).push(p.venue_id);
            });

            const campusToAddedVenue = new Map();
            addedPlacesRaw.forEach(p => {
                if (!campusToAddedVenue.has(p.campus_id)) {
                    campusToAddedVenue.set(p.campus_id, []);
                }
                campusToAddedVenue.get(p.campus_id).push(p.venue_id);
            });

            const replacements = [];
            for (const [campusId, removedVenues] of campusToRemovedVenue.entries()) {
                const addedVenues = campusToAddedVenue.get(campusId);
                if (addedVenues && removedVenues.length === 1 && addedVenues.length === 1) {
                    replacements.push({
                        campusId: campusId,
                        fromVenueId: removedVenues[0],
                        toVenueId: addedVenues[0]
                    });
                }
            }

            if (replacements.length > 0) {
                teachers.value.forEach(teacher => {
                    if (teacher.scheduled && teacher.scheduled.length > 0) {
                        teacher.scheduled.forEach(schedule => {
                            if (schedule.course_id === updatedCourse.id) {
                                replacements.forEach(rep => {
                                    if (schedule.campus_id === rep.campusId && schedule.venue_id === rep.fromVenueId) {
                                        schedule.venue_id = rep.toVenueId;
                                    }
                                });
                            }
                        });
                    }
                });
            }
            
            courses.value[index] = { ...courses.value[index], ...updatedCourse };
        }
    };

    const deleteCourse = (courseId) => {
        courses.value = courses.value.filter(c => c.id !== courseId);
        teachers.value.forEach(teacher => {
            teacher.teaches = teacher.teaches.filter(teachCourseId => teachCourseId !== courseId);
            if (teacher.scheduled) {
                teacher.scheduled = teacher.scheduled.filter(s => s.course_id !== courseId);
            }
        });
    };

    const addCampus = (campusData) => {
        const { id, ...data } = campusData;
        const newCampus = { ...data, id: uuidv4(), venues: campusData.venues || [] };
        campuses.value.push(newCampus);
    };
    const updateCampus = (updatedCampus) => {
        const index = campuses.value.findIndex(c => c.id === updatedCampus.id);
        if (index !== -1) {
            campuses.value[index] = { ...campuses.value[index], ...updatedCampus };
        }
    };
    const deleteCampus = (campusId) => {
        campuses.value = campuses.value.filter(c => c.id !== campusId);
        courses.value.forEach(course => {
            course.place = course.place.filter(p => p.campus_id !== campusId);
        });
        if (selectedCampusIdForCampusView.value === campusId) {
            selectedCampusIdForCampusView.value = null;
        }
        teachers.value.forEach(teacher => {
            if (teacher.scheduled) {
                teacher.scheduled = teacher.scheduled.filter(s => s.campus_id !== campusId);
            }
        });
    };

    const addVenueToCampus = (campusId, venueData) => {
        const campus = campuses.value.find(c => c.id === campusId);
        if (campus) {
            const { id, ...data } = venueData;
            const newVenue = { ...data, id: uuidv4() };
            if (!Array.isArray(campus.venues)) {
                campus.venues = [];
            }
            campus.venues.push(newVenue);
        }
    };
    const updateVenueInCampus = (campusId, updatedVenue) => {
        const campus = campuses.value.find(c => c.id === campusId);
        if (campus && Array.isArray(campus.venues)) {
            const venueIndex = campus.venues.findIndex(v => v.id === updatedVenue.id);
            if (venueIndex !== -1) {
                campus.venues[venueIndex] = { ...campus.venues[venueIndex], ...updatedVenue };
            }
        }
    };
    const deleteVenueFromCampus = (campusId, venueId) => {
        const campus = campuses.value.find(c => c.id === campusId);
        if (campus && Array.isArray(campus.venues)) {
            campus.venues = campus.venues.filter(v => v.id !== venueId);
            courses.value.forEach(course => {
                course.place = course.place.filter(p => !(p.campus_id === campusId && p.venue_id === venueId));
            });
            teachers.value.forEach(teacher => {
                if (teacher.scheduled) {
                    teacher.scheduled = teacher.scheduled.filter(s => !(s.campus_id === campusId && s.venue_id === venueId));
                }
            });
        }
    };

    const updateCampusScheduleDensity = (campusId, dayId, timeId, count) => {
        const campus = campuses.value.find(c => c.id === campusId);
        if (!campus) return;

        if (!Array.isArray(campus.schedule_density)) {
            campus.schedule_density = [];
        }

        const densityIndex = campus.schedule_density.findIndex(
            d => d.day_id === dayId && d.time_id === timeId
        );

        const newCount = Math.max(0, count || 0);

        if (densityIndex !== -1) {
            if (newCount === 0) {
                campus.schedule_density.splice(densityIndex, 1);
            } else {
                campus.schedule_density[densityIndex].count = newCount;
            }
        } else if (newCount > 0) {
            campus.schedule_density.push({ day_id: dayId, time_id: timeId, count: newCount });
        }
    };

    const getExpectedCountForCampusCell = computed(() => (campusId, dayId, timeId) => {
        const campus = campuses.value.find(c => c.id === campusId);
        if (!campus || !campus.schedule_density) {
            return 0;
        }
        const density = campus.schedule_density.find(
            d => d.day_id === dayId && d.time_id === timeId
        );
        return density ? density.count : 0;
    });

    const teacherOptions = computed(() => teachers.value.map(t => ({ label: t.name, value: t.id })));

    const getScheduleMapForTeacher = computed(() => (teacherId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (!teacher || !teacher.scheduled) {
            return new Map();
        }
        const scheduleMap = new Map();
        teacher.scheduled.forEach(s => {
            const key = `${s.day_id}-${s.time_id}`;
            scheduleMap.set(key, s);
        });
        return scheduleMap;
    });

    const getUnavailableMapForTeacher = computed(() => (teacherId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (!teacher || !teacher.unavailable) {
            return new Set();
        }
        const unavailableSet = new Set();
        teacher.unavailable.forEach(slot => {
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
        teachers.value.forEach(teacher => {
            if (teacher.scheduled) {
                teacher.scheduled.forEach(schedule => {
                    const campusMatch = schedule.campus_id === campusId;
                    const venueMatch = !venueId || schedule.venue_id === venueId;

                    if (campusMatch && venueMatch) {
                        const key = `${schedule.day_id}-${schedule.time_id}`;
                        if (!scheduleMap.has(key)) {
                            scheduleMap.set(key, []);
                        }
                        scheduleMap.get(key).push({ schedule, teacher });
                    }
                });
            }
        });
        return scheduleMap;
    });

    const teacherCourseOptions = computed(() => (teacherId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (!teacher) return [];
        return teacher.teaches
            .map(courseId => courses.value.find(c => c.id === courseId))
            .filter(Boolean)
            .map(course => ({ label: course.name, value: course.id }));
    });

    const courseCampusOptions = computed(() => (courseId) => {
        const course = courses.value.find(c => c.id === courseId);
        if (!course || !course.place) return [];
        const campusIds = new Set(course.place.map(p => p.campus_id));
        return campuses.value
            .filter(campus => campusIds.has(campus.id))
            .map(campus => ({ label: campus.name, value: campus.id }));
    });

    const courseVenueOptions = computed(() => (courseId, campusId) => {
        const course = courses.value.find(c => c.id === courseId);
        if (!course || !course.place) return [];
        const venueIds = new Set(
            course.place
                .filter(p => p.campus_id === campusId)
                .map(p => p.venue_id)
        );
        const campus = campuses.value.find(c => c.id === campusId);
        if (!campus || !campus.venues) return [];
        return campus.venues
            .filter(venue => venueIds.has(venue.id))
            .map(venue => ({ label: venue.name, value: venue.id }));
    });

    const addSchedule = (teacherId, scheduleData) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (teacher) {
            const newSchedule = { ...scheduleData, id: uuidv4() };
            if (!Array.isArray(teacher.scheduled)) {
                teacher.scheduled = [];
            }
            teacher.scheduled.push(newSchedule);
        }
    };

    const updateSchedule = (teacherId, updatedSchedule) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (teacher && Array.isArray(teacher.scheduled)) {
            const index = teacher.scheduled.findIndex(s => s.id === updatedSchedule.id);
            if (index !== -1) {
                teacher.scheduled[index] = { ...teacher.scheduled[index], ...updatedSchedule };
            }
        }
    };

    const deleteSchedule = (teacherId, scheduleId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (teacher && Array.isArray(teacher.scheduled)) {
            teacher.scheduled = teacher.scheduled.filter(s => s.id !== scheduleId);
        }
    };

    const toggleUnavailableSlot = (teacherId, dayId, timeId) => {
        const teacher = teachers.value.find(t => t.id === teacherId);
        if (!teacher) return;
        if (!Array.isArray(teacher.unavailable)) {
            teacher.unavailable = [];
        }

        const key = `${dayId}-${timeId}`;
        const index = teacher.unavailable.findIndex(s => s.day_id === dayId && s.time_id === timeId);

        if (index !== -1) {
            teacher.unavailable.splice(index, 1);
        } else {
            teacher.unavailable.push({ day_id: dayId, time_id: timeId });
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
        teachers.value.forEach(teacher => {
            if (teacher.scheduled) {
                teacher.scheduled = teacher.scheduled.filter(s => s.time_id !== timeSlotId);
            }
        });
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
        teachers.value.forEach(teacher => {
            if (teacher.scheduled) {
                teacher.scheduled = teacher.scheduled.filter(s => s.day_id !== dayId);
            }
        });
    };

    return {
        isInitialized,
        isSolving,
        teachers,
        courses,
        campuses,
        time,
        day,
        initializeData,
        replaceAllData,
        revertChanges,
        commitChanges,

        getUnavailableMapForTeacher,
        toggleUnavailableSlot,

        selectedCampusIdForCampusView,
        selectedTeacherIdForTeacherView,
        selectedVenueIdForCampusView,

        courseOptions,
        campusOptions,
        venueOptionsByCampus,

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