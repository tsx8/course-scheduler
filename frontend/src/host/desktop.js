const wailsWindow = () => window?.runtime;
const wailsApp = () => window?.go?.backend?.App;

export const isWailsRuntime = () => typeof window !== 'undefined' && !!wailsWindow() && !!wailsApp();

const ensureWailsApp = () => {
    const app = wailsApp();
    if (!app) {
        throw new Error('Wails runtime is not available.');
    }
    return app;
};

const ensureWailsWindow = () => {
    const runtime = wailsWindow();
    if (!runtime) {
        throw new Error('Wails window runtime is not available.');
    }
    return runtime;
};

const normaliseDialogResult = (value) => {
    if (value === '') {
        return null;
    }
    return value ?? null;
};

const wailsCommandMap = {
    has_unsaved_changes: () => ensureWailsApp().HasUnsavedChanges(),
    save_temp_data: ({ content }) => ensureWailsApp().SaveTempData(content),
    load_data: () => ensureWailsApp().LoadData(),
    clear_temp_data: () => ensureWailsApp().ClearTempData(),
    commit_data: () => ensureWailsApp().CommitData(),
    list_committed_teachers: () => ensureWailsApp().ListCommittedTeachers(),
    run_solver: () => ensureWailsApp().RunSolver(),
    finalize_and_close: ({ save }) => ensureWailsApp().FinalizeAndClose(save),
    import_json: ({ filePath }) => ensureWailsApp().ImportJSON(filePath),
    import_database: ({ filePath }) => ensureWailsApp().ImportDatabase(filePath),
    export_database: ({ filePath }) => ensureWailsApp().ExportDatabase(filePath),
    export_json: ({ filePath }) => ensureWailsApp().ExportJSON(filePath)
};

export const invoke = async (command, payload = {}) => {
    const handler = wailsCommandMap[command];
    if (!handler) {
        throw new Error(`Unsupported Wails command: ${command}`);
    }
    return handler(payload);
};

export const listen = async (eventName, callback) => {
    ensureWailsWindow().EventsOn(eventName, (...data) => callback({ payload: data[0] }));
    return () => ensureWailsWindow().EventsOff(eventName);
};

export const emit = async (eventName, payload) => {
    ensureWailsWindow().EventsEmit(eventName, payload);
};

export const open = async (options = {}) => {
    const result = await ensureWailsApp().OpenDialog({
        title: options.title || '',
        defaultPath: options.defaultPath || '',
        filters: (options.filters || []).map((filter) => ({
            displayName: filter.name,
            pattern: (filter.extensions || []).map((extension) => `*.${extension}`).join(';')
        })),
        multiple: !!options.multiple
    });
    return normaliseDialogResult(result);
};

export const save = async (options = {}) => {
    const result = await ensureWailsApp().SaveDialog({
        title: options.title || '',
        defaultPath: options.defaultPath || '',
        filters: (options.filters || []).map((filter) => ({
            displayName: filter.name,
            pattern: (filter.extensions || []).map((extension) => `*.${extension}`).join(';')
        }))
    });
    return normaliseDialogResult(result);
};

export const getCurrentWindow = () => ({
    minimize: async () => {
        ensureWailsWindow().WindowMinimise();
    },
    toggleMaximize: async () => {
        ensureWailsWindow().WindowToggleMaximise();
    },
    close: async () => {
        ensureWailsWindow().Quit();
    },
    isMaximized: async () => ensureWailsWindow().WindowIsMaximised(),
    onResized: async (callback) => {
        const listener = () => callback();
        window.addEventListener('resize', listener);
        return () => window.removeEventListener('resize', listener);
    },
    listen: async (eventName, callback) => listen(eventName, callback)
});
