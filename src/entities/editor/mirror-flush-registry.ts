import type { ProjectId, TabId } from '@shared/api/bindings'

type FlushFn = () => Promise<void> | void

/**
 * A registered flush plus the project whose mirror it writes to.
 *
 * The project is carried because the flush handshake is no longer app-exit-only: closing one project
 * asks every window to flush *that project's* models before its `file_mirror_dirty` writes stop being
 * accepted (`FlushScope::Project`, audit wave 2 #12). Without the key a window rendering two projects
 * would either flush both — pointless writes while the user closes one — or flush none.
 *
 * The window axis needs no key of its own: each OS window is a separate JS realm with its own copy of
 * this module, so everything in these maps already belongs to the window reading it. That is why
 * `FlushScope::Window` is answered by a label comparison in `hot-exit-flush-provider.tsx` rather than
 * by a lookup here.
 */
type RegisteredFlush = { projectId: ProjectId; flush: FlushFn }

const flushersByTabId = new Map<TabId, RegisteredFlush>()

export const registerMirrorFlush = (tabId: TabId, projectId: ProjectId, flush: FlushFn) => {
    flushersByTabId.set(tabId, { projectId, flush })
}

export const unregisterMirrorFlush = (tabId: TabId) => {
    flushersByTabId.delete(tabId)
}

/**
 * Same one-callback-per-tab shape as `flushersByTabId` above, kept as its own map instead of being
 * folded into it. `use-editor-file-persistence` (the hot-exit *mirror*, i.e. unsaved draft content)
 * and `use-editor-view-state` (monaco `viewState` — cursor/scroll) both register a flush keyed by
 * the exact same `tabId` for every open file tab; a single shared map would have the second hook's
 * registration silently overwrite the first's, dropping unsaved-edit recovery for every tab that
 * also has a mounted `useEditorViewState`.
 */
const viewStateFlushersByTabId = new Map<TabId, RegisteredFlush>()

export const registerViewStateFlush = (tabId: TabId, projectId: ProjectId, flush: FlushFn) => {
    viewStateFlushersByTabId.set(tabId, { projectId, flush })
}

export const unregisterViewStateFlush = (tabId: TabId) => {
    viewStateFlushersByTabId.delete(tabId)
}

const runFlush = async ({ flush }: RegisteredFlush) => {
    try {
        await flush()
    } catch {
        return undefined
    }
}

const registeredFlushes = () => [...flushersByTabId.values(), ...viewStateFlushersByTabId.values()]

const runFlushes = async (entries: RegisteredFlush[]) => {
    await Promise.all(entries.map(runFlush))
}

/**
 * Invokes every registered hot-exit flush — both the file-mirror ones and the viewState ones (see
 * `viewStateFlushersByTabId` above) — and waits for all of them to settle. A flush that throws
 * (synchronously or asynchronously) is swallowed so one stuck pane cannot block the rest from being
 * persisted before the app exits.
 */
export const flushAllMirrors = async () => {
    await runFlushes(registeredFlushes())
}

/**
 * {@link flushAllMirrors} narrowed to one project — what a `FlushScope::Project` request (a project
 * being closed) asks of every window. Panes belonging to other projects are left alone: their mirrors
 * are not about to stop accepting writes, and flushing them would spend IPC round trips inside the
 * handshake's timeout budget on work nothing is waiting for.
 */
export const flushProjectMirrors = async (projectId: ProjectId) => {
    await runFlushes(registeredFlushes().filter((entry) => entry.projectId === projectId))
}
