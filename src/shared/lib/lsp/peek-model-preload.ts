import type { FileSizeTier } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'
import type { Monaco } from '@shared/lib/lsp/monaco-types'

/** Max distinct target files preloaded per single definition/references response. */
export const PEEK_MODEL_PRELOAD_LIMIT = 8

const PEEK_MODEL_PRELOAD_DISPOSE_DELAY_MS = 60_000

const PEEK_MODEL_PRELOAD_ELIGIBLE_TIERS: readonly FileSizeTier[] = ['normal']

export type PeekModelPreloadFile = { content: string; languageId: string; tier: FileSizeTier }
export type PeekModelPreloadReader = (path: string) => Promise<PeekModelPreloadFile>
export type PeekModelPreloadOptions = { readFile?: PeekModelPreloadReader; disposeDelayMs?: number }

type PreloadedModel = ReturnType<Monaco['editor']['createModel']>

const readPeekPreloadFile: PeekModelPreloadReader = (path) => unwrapResult(commands.fileOpen(path))

const preloadedModelTimers = new Map<string, ReturnType<typeof setTimeout>>()

/**
 * Paths whose model was created by this module and has not yet been adopted by a real editing
 * surface (`model-registry.ts`'s `getOrCreateModel`, when a tab actually opens the file). The
 * workspace-edit applier consults this to tell a peek-only preview model apart from a model that
 * genuinely represents an open, saved-through document — a peek preview has no tab, no dirty
 * tracking, and (via the TTL below) no guaranteed lifetime, so an edit landing on it must go
 * through the file IPC write path instead of `pushEditOperations`.
 */
const preloadedModelPaths = new Set<string>()

/** Whether `path` currently has a model created by peek preload that no real editing surface has adopted yet. */
export const isPeekPreloadedModel = (path: string) => preloadedModelPaths.has(path)

/**
 * Cancels the pending auto-dispose timer for `path` and stops treating its model as a peek-only
 * preview. Must be called by whatever takes real ownership of a preloaded model (adopting it into
 * an open tab) — otherwise the TTL below can dispose a model something else now depends on out
 * from under it, or the applier can keep routing edits through the file IPC path for a model that
 * is actually a live, saved-through document.
 */
export const cancelPeekModelDispose = (path: string) => {
    const existingTimer = preloadedModelTimers.get(path)
    if (existingTimer) clearTimeout(existingTimer)
    preloadedModelTimers.delete(path)
    preloadedModelPaths.delete(path)
}

/**
 * Picks which of the given target paths still need a preloaded model, deduplicated and capped
 * at `limit`. Paths for which `hasModel` already returns true (an open tab or an earlier preload)
 * are skipped.
 */
export const selectPeekPreloadPaths = (paths: readonly string[], hasModel: (path: string) => boolean, limit = PEEK_MODEL_PRELOAD_LIMIT) => {
    const distinct = [...new Set(paths)].filter((path) => !hasModel(path))
    return distinct.slice(0, limit)
}

const scheduleModelDispose = (monaco: Monaco, path: string, model: PreloadedModel, disposeDelayMs: number) => {
    const existingTimer = preloadedModelTimers.get(path)
    if (existingTimer) clearTimeout(existingTimer)

    const timer = setTimeout(() => {
        preloadedModelTimers.delete(path)
        preloadedModelPaths.delete(path)
        if (model.isDisposed()) return
        const isAttachedToAnEditor = monaco.editor.getEditors().some((editor) => editor.getModel() === model)
        if (isAttachedToAnEditor) return
        model.dispose()
    }, disposeDelayMs)

    preloadedModelTimers.set(path, timer)
}

/**
 * Creates a monaco model for `path` ahead of time so monaco's Peek widget (which rejects preview
 * requests for uris with no registered model) can render a preview without the target file being
 * open in a tab. No-ops when a model already exists, the file is not `normal` tier (large/readOnly/
 * refused files are never preloaded), or the read fails. The created model auto-disposes after a
 * bounded delay unless an editor is actively displaying it.
 */
export const preloadPeekModel = async (monaco: Monaco, path: string, options: PeekModelPreloadOptions = {}) => {
    const { readFile = readPeekPreloadFile, disposeDelayMs = PEEK_MODEL_PRELOAD_DISPOSE_DELAY_MS } = options
    const uri = monaco.Uri.file(path)
    if (monaco.editor.getModel(uri)) return

    const file = await readFile(path)
    if (!PEEK_MODEL_PRELOAD_ELIGIBLE_TIERS.includes(file.tier)) return
    if (monaco.editor.getModel(uri)) return

    const model = monaco.editor.createModel(file.content, file.languageId, uri)
    preloadedModelPaths.add(path)
    scheduleModelDispose(monaco, path, model, disposeDelayMs)
}

/** Preloads models for up to `PEEK_MODEL_PRELOAD_LIMIT` distinct target paths. Failures per-file are swallowed. */
export const preloadPeekModels = async (monaco: Monaco, paths: readonly string[], options: PeekModelPreloadOptions = {}) => {
    const targets = selectPeekPreloadPaths(paths, (path) => monaco.editor.getModel(monaco.Uri.file(path)) !== null)
    await Promise.all(targets.map((path) => preloadPeekModel(monaco, path, options).catch(() => undefined)))
}

/**
 * Test-only: clears every pending dispose timer and preload-tracking entry. `preloadedModelTimers`/
 * `preloadedModelPaths` are process-wide singletons, and `bun test` does not isolate modules per
 * test file — a preload left registered by one test (the real 60s default TTL never expires within
 * a test run) would otherwise leak into every test file that runs afterward and consults
 * `isPeekPreloadedModel` (e.g. `workspace-edit-applier.test.ts`). Call from an `afterEach` in any
 * test file that preloads a model (directly or via `definition.ts`/`references.ts`'s adapters).
 */
export const resetPeekModelPreloadStateForTests = () => {
    for (const timer of preloadedModelTimers.values()) clearTimeout(timer)
    preloadedModelTimers.clear()
    preloadedModelPaths.clear()
}
