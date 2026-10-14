export const createWorkerContext = async (destinationUrl) => {
    const remap = (source, url) =>
        source.replace(/(from\s+|import\s+)'(\.[^']+)'/g, (_, prefix, path) => `${prefix}${JSON.stringify(new URL(path, url).href)}`)
    const WORKER_URL = new URL('../../../../node_modules/monaco-editor/esm/vs/languages/features/json/jsonWorker.js', import.meta.url)
    const WORKER_PATCH = destinationUrl
    const workerSource = await Bun.file(WORKER_URL).text()
    const WORKER_MARKER = 'this._languageService = getLanguageService({'
    if (workerSource.split(WORKER_MARKER).length !== 2) throw new Error('Unexpected original worker context marker')
    await Bun.write(
        WORKER_PATCH,
        `${remap(workerSource.replace(WORKER_MARKER, 'this._languageService = captureLanguageService({'), WORKER_URL)}\nlet capturedContext;\nconst captureLanguageService = (params) => { capturedContext=params.workspaceContext; return getLanguageService(params); };\nexport { capturedContext };\n`,
    )
    const workerModule = await import(WORKER_PATCH.href)
    const worker = new workerModule.JSONWorker(
        { getMirrorModels: () => [] },
        { languageSettings: { schemas: [] }, languageId: 'json', enableSchemaRequest: false },
    )
    return { worker, context: workerModule.capturedContext }
}
