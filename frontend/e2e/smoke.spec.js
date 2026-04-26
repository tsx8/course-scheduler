import { expect, test } from '@playwright/test';

const emptyScheduleData = {
  teachers: [],
  courses: [],
  campuses: [],
  venues: [],
  time: [],
  day: [],
  course_venues: [],
  teacher_courses: [],
  teacher_campuses: [],
  scheduled_classes: [],
  teacher_unavailability: [],
  schedule_density: [],
};

test.beforeEach(async ({ page }) => {
  await page.addInitScript((data) => {
    window.runtime = {
      EventsOn: () => {},
      EventsOff: () => {},
      EventsEmit: () => {},
      WindowMinimise: () => {},
      WindowToggleMaximise: () => {},
      WindowIsMaximised: () => false,
      Quit: () => {},
    };

    window.go = {
      backend: {
        App: {
          HasUnsavedChanges: async () => false,
          SaveTempData: async () => {},
          LoadData: async () => data,
          ClearTempData: async () => {},
          CommitData: async () => {},
          ListCommittedTeachers: async () => [],
          RunSolver: async () => data,
          FinalizeAndClose: async () => {},
          OpenDialog: async () => '',
          SaveDialog: async () => '',
          ImportJSON: async () => ({ teachers: 0, courses: 0, schedules: 0 }),
          ImportDatabase: async () => ({ teachers: 0, courses: 0, schedules: 0 }),
          ExportDatabase: async () => {},
          ExportJSON: async () => {},
        },
      },
    };
  }, emptyScheduleData);
});

test('loads the desktop shell', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: '排课管理系统' })).toBeVisible();
  await expect(page.getByRole('menu').getByText('校区总课表')).toBeVisible();
});
