import { createRouter, createWebHistory } from 'vue-router';
import MainLayout from '../layouts/MainLayout.vue';
import { useAuthStore } from '../stores/auth';

const routes = [
  {
    path: '/',
    component: MainLayout,
    redirect: '/campus-timetable',
    meta: { requiresAuth: true },
    children: [
      {
        path: 'login',
        name: 'Login',
        component: () => import('../pages/Login.vue'),
        meta: { requiresAuth: false, title: '登录' }
      },
      {
        path: 'campus-timetable',
        name: 'CampusTimetable',
        component: () => import('../pages/CampusTimetable.vue'),
        meta: { title: '校区总课表', requiresAuth: true, requiresScheduler: true }
      },
      {
        path: 'teacher-timetable',
        name: 'TeacherTimetable',
        component: () => import('../pages/TeacherTimetable.vue'),
        meta: { title: '教师个人课表', requiresAuth: true }
      },
      {
        path: 'teacher-management',
        name: 'TeacherManagement',
        component: () => import('../pages/TeacherManagement.vue'),
        meta: { title: '教师信息管理', requiresAuth: true, requiresScheduler: true }
      },
      {
        path: 'course-management',
        name: 'CourseManagement',
        component: () => import('../pages/CourseManagement.vue'),
        meta: { title: '课程信息管理', requiresAuth: true, requiresScheduler: true }
      },
      {
        path: 'venue-management',
        name: 'VenueManagement',
        component: () => import('../pages/VenueManagement.vue'),
        meta: { title: '场地/校区信息管理', requiresAuth: true, requiresScheduler: true }
      },
      {
        path: 'user-management',
        name: 'UserManagement',
        component: () => import('../pages/UserManagement.vue'),
        meta: { title: '用户管理', requiresAuth: true, requiresScheduler: true }
      },
      {
        path: 'audit-logs',
        name: 'AuditLogs',
        component: () => import('../pages/AuditLogs.vue'),
        meta: { title: '审计日志', requiresAuth: true, requiresScheduler: true }
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../pages/Settings.vue'),
        meta: { title: '应用设置', requiresAuth: true }
      },
    ]
  }
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes
});

router.beforeEach(async (to, from, next) => {
  const authStore = useAuthStore();

  if (!authStore.isAuthenticated && localStorage.getItem('session_id')) {
    try {
      await authStore.restoreSession();
    } catch (e) {
      console.error("Session restore failed", e);
    }
  }

  const requiresAuth = to.meta.requiresAuth !== false;

  if (requiresAuth && !authStore.isAuthenticated) {
    next('/login');
  } else if (to.path === '/login' && authStore.isAuthenticated) {
    if (authStore.isScheduler) {
      next('/campus-timetable');
    } else {
      next('/teacher-timetable');
    }
  } else if (to.meta.requiresScheduler && !authStore.isScheduler) {
    if (authStore.isTeacher) {
      next('/teacher-timetable');
    } else {
      next('/');
    }
  } else {
    next();
  }
});

router.afterEach((to) => {
  document.title = `${to.meta.title ? to.meta.title + ' - ' : ''}排课管理系统`;
});

export default router;

