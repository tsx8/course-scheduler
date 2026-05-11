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

const stagedScheduleData = {
  teachers: [{ id: 'teacher-1', name: '张老师', max_teaching_hours: 8 }],
  courses: [{ id: 'course-1', name: '线性代数' }],
  campuses: [{ id: 'campus-1', name: '主校区' }],
  venues: [{ id: 'venue-1', campus_id: 'campus-1', name: 'A101', capacity: 4 }],
  time: [{ id: 'time-1', value: '第一节', corresponding_hours: 1 }],
  day: [{ id: 'day-1', value: '周一' }],
  course_venues: [{ course_id: 'course-1', venue_id: 'venue-1' }],
  teacher_courses: [{ teacher_id: 'teacher-1', course_id: 'course-1' }],
  teacher_campuses: [{ teacher_id: 'teacher-1', campus_id: 'campus-1' }],
  scheduled_classes: [
    {
      id: 'schedule-1',
      teacher_id: 'teacher-1',
      course_id: 'course-1',
      campus_id: 'campus-1',
      venue_id: 'venue-1',
      day_id: 'day-1',
      time_id: 'time-1',
      is_locked: true,
      is_staged: true,
      staged_order: 1,
    },
  ],
  teacher_unavailability: [],
  schedule_density: [],
};

const activeScheduleData = {
  ...stagedScheduleData,
  scheduled_classes: stagedScheduleData.scheduled_classes.map((schedule) => ({
    ...schedule,
    is_staged: false,
    staged_order: 0,
  })),
};

const dragHoverScheduleData = {
  teachers: [
    { id: 'teacher-1', name: '张老师', max_teaching_hours: 8 },
    { id: 'teacher-2', name: '李老师', max_teaching_hours: 8 },
  ],
  courses: [
    { id: 'course-1', name: '线性代数' },
    { id: 'course-2', name: '概率论' },
  ],
  campuses: stagedScheduleData.campuses,
  venues: [...stagedScheduleData.venues, { id: 'venue-2', campus_id: 'campus-1', name: 'B102', capacity: 4 }],
  time: [
    { id: 'time-1', value: '第一节', corresponding_hours: 1 },
    { id: 'time-2', value: '第二节', corresponding_hours: 1 },
  ],
  day: stagedScheduleData.day,
  course_venues: [
    { course_id: 'course-1', venue_id: 'venue-1' },
    { course_id: 'course-2', venue_id: 'venue-2' },
  ],
  teacher_courses: [
    { teacher_id: 'teacher-1', course_id: 'course-1' },
    { teacher_id: 'teacher-2', course_id: 'course-2' },
  ],
  teacher_campuses: [
    { teacher_id: 'teacher-1', campus_id: 'campus-1' },
    { teacher_id: 'teacher-2', campus_id: 'campus-1' },
  ],
  scheduled_classes: [
    { ...activeScheduleData.scheduled_classes[0] },
    {
      id: 'schedule-2',
      teacher_id: 'teacher-2',
      course_id: 'course-2',
      campus_id: 'campus-1',
      venue_id: 'venue-2',
      day_id: 'day-1',
      time_id: 'time-2',
      is_locked: true,
      is_staged: false,
      staged_order: 0,
    },
  ],
  teacher_unavailability: [],
  schedule_density: [],
};

const sameSlotDistinctVenueCardDropData = {
  ...dragHoverScheduleData,
  scheduled_classes: dragHoverScheduleData.scheduled_classes.map((schedule) =>
    schedule.id === 'schedule-2' ? { ...schedule, time_id: 'time-1' } : schedule
  ),
};

const selectHeaderOption = async (page, index, text) => {
  const select = page.locator('.timetable-page__header .n-select').nth(index);
  const option = page.locator('.n-base-select-option:visible').filter({ hasText: text });

  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (!(await option.isVisible().catch(() => false))) {
      await select.click({ force: true });
    }

    try {
      await expect(option).toBeVisible({ timeout: 1000 });
      await option.click({ force: true });
      return;
    } catch {
      await page.keyboard.press('Escape');
    }
  }

  await select.click({ force: true });
  await expect(option).toBeVisible();
  await option.click({ force: true });
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

    const currentData = () => window.__COURSE_SCHEDULER_E2E_DATA || data;

    window.go = {
      backend: {
        App: {
          HasUnsavedChanges: async () => false,
          SaveTempData: async () => {},
          LoadData: async () => currentData(),
          ClearTempData: async () => {},
          CommitData: async () => {},
          ListCommittedTeachers: async () => [],
          RunSolver: async () => currentData(),
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

test('dragging a staged schedule does not select card text', async ({ page }) => {
  await page.addInitScript((data) => {
    window.__COURSE_SCHEDULER_E2E_DATA = data;
  }, stagedScheduleData);

  await page.goto('/campus-timetable');

  const stagedCard = page.locator('.schedule-staging-tray .schedule-card').first();
  await expect(stagedCard.getByText('线性代数')).toBeVisible();
  await expect(stagedCard.getByText('张老师 · 周一 第一节')).toBeVisible();
  await expect(stagedCard.getByText('主校区 · A101')).toBeVisible();
  await expect(stagedCard.getByText('暂存', { exact: true })).toHaveCount(0);
  await expect(stagedCard.getByText('已锁定')).toHaveCount(0);
  await expect(stagedCard.getByRole('button', { name: '还原课程' })).toBeVisible();
  await expect(stagedCard.getByRole('button', { name: '取消锁定' })).toHaveCount(0);
  await expect(stagedCard.getByRole('button', { name: '锁定课程' })).toHaveCount(0);
  await expect(stagedCard.getByRole('button', { name: '删除' })).toHaveCount(0);
  const box = await stagedCard.boundingBox();
  if (!box) throw new Error('staged schedule card is not visible');

  await page.mouse.move(box.x + 24, box.y + 24);
  await page.mouse.down();
  await page.mouse.move(box.x + 170, box.y + 44, { steps: 8 });

  await expect(page.locator('.schedule-staging-tray')).toHaveClass(/schedule-staging-tray--dragging/);
  const selectedText = await page.evaluate(() => window.getSelection()?.toString() || '');
  expect(selectedText).toBe('');

  await page.mouse.up();

  await stagedCard.getByRole('button', { name: '还原课程' }).click();
  await expect(page.locator('.schedule-staging-tray')).toHaveCount(0);
});

test('campus timetable schedule cards expose a delete icon action', async ({ page }) => {
  await page.addInitScript((data) => {
    window.__COURSE_SCHEDULER_E2E_DATA = data;
  }, activeScheduleData);

  await page.goto('/campus-timetable');

  await page.locator('.timetable-page__header .n-select').nth(1).click();
  await page.getByText('主校区').click();

  const scheduleCard = page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' }).first();
  await expect(scheduleCard).toBeVisible();

  await scheduleCard.getByRole('button', { name: '删除' }).click();
  await expect(page.getByText('确认删除排课')).toBeVisible();
  await page.getByRole('dialog').getByRole('button', { name: '删除' }).click();
  await expect(scheduleCard).toHaveCount(0);
});

test('campus timetable can filter schedules by course', async ({ page }) => {
  await page.addInitScript((data) => {
    window.__COURSE_SCHEDULER_E2E_DATA = data;
  }, dragHoverScheduleData);

  await page.goto('/campus-timetable');

  await selectHeaderOption(page, 1, '主校区');

  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' })).toBeVisible();
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' })).toBeVisible();

  await selectHeaderOption(page, 4, '线性代数');

  await expect(page.getByLabel('期望容量')).toHaveCount(0);

  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' })).toBeVisible();
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' })).toHaveCount(0);
});

test('campus timetable can save apply and delete custom filter views', async ({ page }) => {
  await page.addInitScript((data) => {
    window.__COURSE_SCHEDULER_E2E_DATA = data;
  }, dragHoverScheduleData);

  await page.goto('/campus-timetable');

  await selectHeaderOption(page, 1, '主校区');
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' })).toBeVisible();
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' })).toBeVisible();
  await selectHeaderOption(page, 4, '线性代数');
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' })).toHaveCount(0);

  await page.locator('.timetable-page__header .n-select').first().click();
  await page.getByRole('button', { name: '保存当前筛选' }).click();
  await page.keyboard.press('Escape');

  await selectHeaderOption(page, 4, '概率论');
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' })).toBeVisible();

  await selectHeaderOption(page, 0, '主校区 · 线性代数');
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' })).toBeVisible();
  await expect(page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' })).toHaveCount(0);

  await page.locator('.timetable-page__header .n-select').first().click();
  await page.getByRole('button', { name: '删除当前视图' }).click();
  await page.locator('.timetable-page__header .n-select').first().click();
  await expect(page.locator('.n-base-select-option:visible').filter({ hasText: '主校区 · 线性代数' })).toHaveCount(0);
});

test('drag hover distinguishes timetable cells from target schedule cards', async ({ page }) => {
  await page.addInitScript((data) => {
    window.__COURSE_SCHEDULER_E2E_DATA = data;
  }, dragHoverScheduleData);

  await page.goto('/campus-timetable');

  await page.locator('.timetable-page__header .n-select').nth(1).click();
  await page.getByText('主校区').click();

  const sourceCard = page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' }).first();
  const targetCard = page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' }).first();
  const targetCell = targetCard.locator(
    'xpath=ancestor::*[contains(concat(" ", normalize-space(@class), " "), " schedule-drop-cell ")][1]'
  );

  await expect(sourceCard).toBeVisible();
  await expect(targetCard).toBeVisible();

  const sourceBox = await sourceCard.boundingBox();
  const targetBox = await targetCard.boundingBox();
  const targetCellBox = await targetCell.boundingBox();
  if (!sourceBox || !targetBox || !targetCellBox) throw new Error('drag hover target is not visible');

  await page.mouse.move(sourceBox.x + sourceBox.width / 2, sourceBox.y + sourceBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(targetBox.x + targetBox.width / 2, targetBox.y + targetBox.height / 2, { steps: 8 });

  await expect(targetCard).toHaveClass(/schedule-card--drop-targeted/);
  await expect(targetCell).not.toHaveClass(/schedule-drop-cell--hover/);

  await page.mouse.move(targetCellBox.x + 10, targetCellBox.y + 10, { steps: 5 });

  await expect(targetCell).toHaveClass(/schedule-drop-cell--hover/);
  await expect(targetCard).not.toHaveClass(/schedule-card--drop-targeted/);

  await page.mouse.up();
});

test('same-slot campus card swap is disabled when positions do not change', async ({ page }) => {
  await page.addInitScript((data) => {
    window.__COURSE_SCHEDULER_E2E_DATA = data;
  }, sameSlotDistinctVenueCardDropData);

  await page.goto('/campus-timetable');

  await page.locator('.timetable-page__header .n-select').nth(1).click();
  await page.getByText('主校区').click();

  const sourceCard = page.locator('.timetable-scroll .schedule-card').filter({ hasText: '线性代数' }).first();
  const targetCard = page.locator('.timetable-scroll .schedule-card').filter({ hasText: '概率论' }).first();

  await expect(sourceCard).toBeVisible();
  await expect(targetCard).toBeVisible();

  const sourceBox = await sourceCard.boundingBox();
  const targetBox = await targetCard.boundingBox();
  if (!sourceBox || !targetBox) throw new Error('card drop target is not visible');

  await page.mouse.move(sourceBox.x + sourceBox.width / 2, sourceBox.y + sourceBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(targetBox.x + targetBox.width / 2, targetBox.y + targetBox.height / 2, { steps: 8 });

  await expect(targetCard).toHaveClass(/schedule-card--drop-targeted/);
  await page.mouse.up();

  await expect(page.getByText('目标位置已有排课', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '交换' })).toBeDisabled();
  await expect(page.getByText('当前目标场地不适用于拖拽课程，不能执行换下、交换或覆盖。')).toHaveCount(0);
  await expect(page.getByText('未配置可使用')).toHaveCount(0);
});
