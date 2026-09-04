import type { QueryClient } from '@tanstack/react-query'
import type { MirrorEntry, OpenedFile, ProjectId, ProjectLayout, TabPathMove } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { takeWaitMarkers } from '@entities/agent/agent-wait-marker-registry'
import { releaseWaitMarker } from '@entities/agent/agent.ipc'
import { clearMirror, mirrorDirty, openFile } from '@entities/file/file.ipc'
import { applyTabPathChange } from '@entities/layout/layout.ipc'
import { applyModelLanguage, disposeModel, getModel, retargetModel } from '@entities/editor/model-registry'
import { getOpenWithOverride, setOpenWithOverride } from '@entities/editor/open-with-registry'

/**
 * Everything this module reaches outside the query cache, injected so the migration can be tested
 * without monaco or a backend (same `deps`-with-defaults seam `shared/lib/lsp/workspace-edit-applier.ts`
 * uses).
 */
export type TabPathChangeDeps = {
    applyTabPathChange: typeof applyTabPathChange
    openFile: typeof openFile
    mirrorDirty: typeof mirrorDirty
    clearMirror: typeof clearMirror
    readModelContent: (path: string) => string | null
    retargetModel: typeof retargetModel
    applyModelLanguage: typeof applyModelLanguage
    disposeModel: typeof disposeModel
    getOpenWithOverride: typeof getOpenWithOverride
    setOpenWithOverride: typeof setOpenWithOverride
    takeWaitMarkers: typeof takeWaitMarkers
    releaseWaitMarker: typeof releaseWaitMarker
}

export const defaultTabPathChangeDeps: TabPathChangeDeps = {
    applyTabPathChange,
    openFile,
    mirrorDirty,
    clearMirror,
    readModelContent: (path) => getModel(path)?.getValue() ?? null,
    retargetModel,
    applyModelLanguage,
    disposeModel,
    getOpenWithOverride,
    setOpenWithOverride,
    takeWaitMarkers,
    releaseWaitMarker,
}

type ProjectScope = { queryClient: QueryClient; projectId: ProjectId }

/** Where `QUERY_KEY.LAYOUT.DETAIL` puts the `ProjectId`, derived from the key itself rather than hard-coded so a key-shape change can't silently turn the filter below into a no-op. */
const LAYOUT_DETAIL_PROJECT_ID_KEY_INDEX = QUERY_KEY.LAYOUT.DETAIL('').length - 1

/**
 * Every absolute path the tabs of one project still address through the two bare-path-keyed byte
 * caches: `file` tabs (an editor pane's `FILE.CONTENT`, a preview pane's `FILE.RAW`), `diff` tabs
 * (which read `FILE.CONTENT` for *both* sides, hence `compareWith` too) and `claudeDiff` tabs.
 * Spans every tree the project owns — the main window's and each auxiliary window's — through
 * `collectAllPaneTabs`.
 */
const collectAddressedFilePaths = (layout: ProjectLayout) =>
    collectAllPaneTabs(layout).flatMap((tab) => {
        if (tab.kind.kind === 'file' || tab.kind.kind === 'claudeDiff') return [tab.kind.path]
        if (tab.kind.kind !== 'diff') return []
        return tab.kind.compareWith ? [tab.kind.path, tab.kind.compareWith] : [tab.kind.path]
    })

/**
 * Whether any window of any open project still addresses `path`.
 *
 * `layout` is *this* project's post-close tree and takes precedence over the cache: `useCloseTab`
 * releases the closed path before `applyFreshLayout` publishes the new layout, so this project's
 * cached `LAYOUT.DETAIL` entry is still the pre-close one and would report the tab that was just
 * closed as open. Every *other* project's layout is read straight from the query cache — the shape
 * `ipc-sync-provider.tsx`'s `collectOpenFilePathsOutsideProject` already uses to answer the same
 * "is this path open somewhere else" question for the "reopen with" registry.
 */
const isFilePathAddressedAnywhere = ({
    queryClient,
    projectId,
    path,
    layout,
}: {
    queryClient: QueryClient
    projectId: ProjectId | null
    path: string
    layout: ProjectLayout
}) =>
    collectAddressedFilePaths(layout).includes(path) ||
    queryClient
        .getQueriesData<ProjectLayout>({ queryKey: QUERY_KEY.LAYOUT.ALL })
        .filter(([queryKey]) => queryKey[LAYOUT_DETAIL_PROJECT_ID_KEY_INDEX] !== (projectId ?? ''))
        .some(([, cachedLayout]) => !!cachedLayout && collectAddressedFilePaths(cachedLayout).includes(path))

/**
 * Releases everything the frontend keeps keyed by a file path once no tab addresses it any more —
 * the agent wait markers holding an agent's `ide:diff-requested`-style wait open, the hot-exit
 * mirror, the sticky "reopen with" override, and the monaco model itself.
 *
 * `layout` is the post-close layout: a file open in a split (or in another window) still has a tab,
 * so only the per-path state of a path that is now closed *everywhere* is torn down. Disposing the
 * model is audit §1-6 — nothing but `untitled` tabs ever disposed one, so every file opened in a
 * session kept its full text (and its undo stack) alive in monaco until the app quit. The undo stack
 * is the price: reopening the file starts a fresh model, which is what contract §3 S8 records as
 * superseding the "preserve undo" wish.
 *
 * Called both by `useCloseTab` (one tab closed by hand) and by {@link followDeletedPathInTabs} (every
 * tab a delete closed at once).
 *
 * The `FILE.CONTENT`/`FILE.RAW` reclaim at the end is contract §C.2-4 M3: both queries are
 * `staleTime: Infinity` on a bare path key, so closing a tab used to leave a full second copy of the
 * file — for `FILE.RAW` a preview's raw `ArrayBuffer`, tens of MB for a PDF or an image — sitting in
 * the query cache for the global 10-minute `gcTime`, exactly the buffer audit §1-6 had just taught
 * this function to free from monaco. It is deliberately gated harder than the model dispose above:
 * `stillOpenElsewhere` only asks about *this* project's `file` tabs, while dropping the byte caches
 * has to be certain no diff/claudeDiff side and no other project's window still reads them. Doing it
 * here — the one place a file path is known to be closed everywhere — is also what keeps d-43's
 * save-clobber contract intact: `useSaveFile`'s synchronous `FILE.CONTENT` patch only ever runs for a
 * path that still has an open tab, so this reclaim can never race it into a re-fetch.
 */
export const releaseClosedFileTabPath = (
    { queryClient, projectId, path, layout }: { queryClient: QueryClient; projectId: ProjectId | null; path: string; layout: ProjectLayout },
    deps: TabPathChangeDeps = defaultTabPathChangeDeps,
) => {
    for (const marker of deps.takeWaitMarkers(path)) void deps.releaseWaitMarker(marker)

    if (projectId) {
        void deps.clearMirror({ projectId, path }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
    }

    const stillOpenElsewhere = collectAllPaneTabs(layout).some((tab) => tab.kind.kind === 'file' && tab.kind.path === path)
    if (stillOpenElsewhere) return

    deps.setOpenWithOverride(path, null)
    deps.disposeModel(path)

    if (isFilePathAddressedAnywhere({ queryClient, projectId, path, layout })) return
    queryClient.removeQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path), exact: true })
    queryClient.removeQueries({ queryKey: QUERY_KEY.FILE.RAW(path), exact: true })
}

/**
 * Rewrites the `FILE.MIRRORS` cache for one moved path in a single pass: the old path's entry always
 * goes (that file no longer exists), and the new path's entry appears when a draft was carried over.
 * Returns `undefined` — TanStack Query's "leave the query alone" — when there is nothing to do, and
 * whenever the project's mirror list has never been fetched: seeding a partial list into that
 * `staleTime: Infinity` query would convince every pane in the project that no other mirror exists
 * (the hazard `use-editor-file-persistence.ts`'s `settleDraftToDiskContent` documents). The backend
 * already holds the new path's mirror, so the first real fetch reports it anyway.
 */
const rewriteMirrorCache = ({ queryClient, projectId }: ProjectScope, move: TabPathMove, migrated: MirrorEntry | null) => {
    queryClient.setQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(projectId), (previous) => {
        if (!previous) return undefined
        if (!migrated && !previous.some((entry) => entry.path === move.from)) return undefined
        const rest = previous.filter((entry) => entry.path !== move.from && entry.path !== move.to)
        return migrated ? [...rest, migrated] : rest
    })
}

/**
 * Carries one renamed file's unsaved draft from the old path to the new one through the hot-exit
 * mirror, and drops the old path's mirror either way.
 *
 * The mirror is the only channel that survives the rename: `EditorPane`'s render-phase path-switch
 * reset drops its own `dirty`/`syncedContent` the moment the tab's path changes, after which its
 * disk sync would apply the new path's on-disk text over the moved buffer. Re-mirroring under the
 * new path first means the pane's mirror-restore effect (`use-editor-file-persistence.ts`, keyed on
 * `[editor, path, mirrors]`) finds the draft waiting and re-marks the tab dirty instead — the same
 * recovery path a crash would take.
 *
 * `draft` is resolved by the caller *before* the model moves (afterwards the old path has no model
 * to read) and is preferred over the existing mirror, which lags a 500ms typing debounce.
 *
 * The old path's mirror is dropped **before** the new one is written, not after. Both IPCs key the
 * mirror file by a hash of the *canonicalized* path (`file/service.rs`'s `mirror_file_path` through
 * `ensure_within_root`), and on a case-insensitive filesystem a case-only rename
 * (`readme.md` → `README.md`, which audit §4-A-1 made possible) canonicalizes both sides to the very
 * same file: clearing afterwards would delete the entry that was just written and leave the carried
 * draft recoverable only for the rest of this session.
 */
const migrateMirror = async (scope: ProjectScope, move: TabPathMove, draft: string | null, deps: TabPathChangeDeps) => {
    const { queryClient, projectId } = scope
    const hadMirror = !!queryClient.getQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(projectId))?.some((entry) => entry.path === move.from)

    if (draft === null && !hadMirror) return

    await deps.clearMirror({ projectId, path: move.from }).catch(() => undefined)
    const migrated =
        draft === null
            ? null
            : await deps
                  .mirrorDirty({ projectId, path: move.to, content: draft })
                  .then((diskModifiedMs): MirrorEntry => ({ path: move.to, content: draft, savedAtMs: Date.now(), diskModifiedMs, conflict: false }))
    rewriteMirrorCache(scope, move, migrated)
}

/**
 * Moves every piece of per-path state one renamed file owns: the monaco model (buffer, view state,
 * and the editors displaying it), the "reopen with" override, the `FILE.CONTENT` cache, and the
 * hot-exit mirror.
 *
 * The `FILE.CONTENT` cache is seeded from the old path's entry *before* the re-read lands so the
 * pane never falls into its "still loading" branch on the new path — that branch unmounts
 * `CodeEditor` outright, which would tear down and rebuild the very editor this migration just
 * re-pointed. The authoritative re-read then replaces the seed and re-languages the model, which is
 * what makes a rename that changes the extension (`notes.txt` → `notes.ts`) highlight as its new
 * language.
 *
 * Both are skipped when the old path has no cached content — that means no editor ever read this
 * file (a `PreviewPane` tab reads `FILE.RAW` instead), so there is no model to re-language and no
 * reason to spend a `file_open` re-read on, say, a renamed 30MB PDF. `useRenameEntry`'s own
 * invalidation still refreshes the new path for those tabs.
 */
const migrateRenamedFileTabPath = async (scope: ProjectScope, move: TabPathMove, deps: TabPathChangeDeps) => {
    const { queryClient, projectId } = scope
    const cachedMirrorContent = queryClient
        .getQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(projectId))
        ?.find((entry) => entry.path === move.from)?.content
    const draft = move.dirty ? (deps.readModelContent(move.from) ?? cachedMirrorContent ?? null) : null
    const openWithOverride = deps.getOpenWithOverride(move.from)

    deps.retargetModel(move.from, move.to)

    if (openWithOverride) {
        deps.setOpenWithOverride(move.from, null)
        deps.setOpenWithOverride(move.to, openWithOverride)
    }

    const previousContent = queryClient.getQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(move.from))
    if (previousContent) {
        queryClient.setQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(move.to), { ...previousContent, path: move.to })

        const reopened = await deps.openFile(move.to).catch(() => null)
        if (reopened) {
            queryClient.setQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(move.to), reopened)
            deps.applyModelLanguage(move.to, reopened.languageId)
        }
    }

    await migrateMirror(scope, move, draft, deps)
}

/**
 * Makes open tabs follow a completed `file_rename` — the frontend half of audit §4-B A3. Asks Rust
 * to repoint the tabs (a directory rename moves every tab under it), then migrates each moved path's
 * frontend-only state *before* the caller publishes the new layout, so no pane ever observes its new
 * path with the old path's model, mirror or cached content still in place.
 *
 * Returns the backend result so the caller can feed `result.layout` through `applyFreshLayout`'s
 * revision gate like every other layout mutation.
 */
export const followRenamedPathInTabs = async (
    { queryClient, projectId, from, to }: ProjectScope & { from: string; to: string },
    deps: TabPathChangeDeps = defaultTabPathChangeDeps,
) => {
    const result = await deps.applyTabPathChange({ projectId, change: { kind: 'renamed', from, to } })
    for (const move of result.moved) await migrateRenamedFileTabPath({ queryClient, projectId }, move, deps)
    return result
}

/**
 * Makes open tabs follow a completed `file_delete`: Rust closes every file tab at (or under) the
 * deleted path, and each closed path's frontend state is released exactly as a hand-closed tab's is.
 * Closing — rather than leaving the tab pointing at a file that no longer exists — is the contract
 * §3 S8 decision; a `⌘S` on such a tab would otherwise recreate the deleted file's directory through
 * `write_atomic`'s `create_dir_all`.
 */
export const followDeletedPathInTabs = async (
    { queryClient, projectId, path }: ProjectScope & { path: string },
    deps: TabPathChangeDeps = defaultTabPathChangeDeps,
) => {
    const result = await deps.applyTabPathChange({ projectId, change: { kind: 'deleted', path } })
    for (const closedPath of result.closedPaths) {
        releaseClosedFileTabPath({ queryClient, projectId, path: closedPath, layout: result.layout }, deps)
    }
    return result
}
