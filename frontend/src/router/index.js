import { createRouter, createWebHistory } from 'vue-router';
import MainLayout from '../layouts/MainLayout.vue';

const routes = [
  {
    path: '/',
    component: MainLayout,
    redirect: '/campus-timetable',
    children: [
      {
        path: 'campus-timetable',
        name: 'CampusTimetable',
        component: () => import('../pages/CampusTimetable.vue'),
        meta: { title: '校区总课表' }
      },
      {
        path: 'teacher-timetable',
        name: 'TeacherTimetable',
        component: () => import('../pages/TeacherTimetable.vue'),
        meta: { title: '教师个人课表' }
      },
      {
        path: 'schedule-issues',
        name: 'ScheduleIssues',
        component: () => import('../pages/ScheduleIssues.vue'),
        meta: { title: '问题检查' }
      },
      {
        path: 'teacher-management',
        name: 'TeacherManagement',
        component: () => import('../pages/TeacherManagement.vue'),
        meta: { title: '教师信息管理' }
      },
      {
        path: 'course-management',
        name: 'CourseManagement',
        component: () => import('../pages/CourseManagement.vue'),
        meta: { title: '课程信息管理' }
      },
      {
        path: 'venue-management',
        name: 'VenueManagement',
        component: () => import('../pages/VenueManagement.vue'),
        meta: { title: '场地/校区信息管理' }
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../pages/Settings.vue'),
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
