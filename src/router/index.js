import { createRouter, createWebHistory } from 'vue-router';
import MainLayout from '../layouts/MainLayout.vue';
import TeacherManagement from '../pages/TeacherManagement.vue';
import CourseManagement from '../pages/CourseManagement.vue';
import VenueManagement from '../pages/VenueManagement.vue';
import CampusTimetable from '../pages/CampusTimetable.vue';
import TeacherTimetable from '../pages/TeacherTimetable.vue';
import Settings from '../pages/Settings.vue';

const routes = [
  {
    path: '/',
    component: MainLayout,
    redirect: '/campus-timetable',
    children: [
      {
        path: 'campus-timetable',
        name: 'CampusTimetable',
        component: CampusTimetable,
        meta: { title: '校区总课表' }
      },
      {
        path: 'teacher-timetable',
        name: 'TeacherTimetable',
        component: TeacherTimetable,
        meta: { title: '教师个人课表' }
      },
      {
        path: 'teacher-management',
        name: 'TeacherManagement',
        component: TeacherManagement,
        meta: { title: '教师信息管理' }
      },
      {
        path: 'course-management',
        name: 'CourseManagement',
        component: CourseManagement,
        meta: { title: '课程信息管理' }
      },
      {
        path: 'venue-management',
        name: 'VenueManagement',
        component: VenueManagement,
        meta: { title: '场地/校区信息管理' }
      },
      {
        path: 'settings',
        name: 'Settings',
        component: Settings,
        meta: { title: '应用设置' }
      },
    ]
  }
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes
});

router.afterEach((to) => {
  document.title = `${to.meta.title ? to.meta.title + ' - ' : ''}排课管理系统`;
});

export default router;
