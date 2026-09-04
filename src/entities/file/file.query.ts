import type { QueryClient } from '@tanstack/react-query'
import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { OpenedFile, ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { copyEntry, createEntry, deleteEntry, listMirrors, listUntitledMirrors, openFile, renameEntry, saveFile } from '@entities/file/file.ipc'
import { readFileRaw } from '@entities/file/file.raw'
import { publishFileSaveSettle } from '@entities/editor/file-save-settle-registry'
import { applyFreshLayout } from '@entities/layout/layout.query'
import { followDeletedPathInTabs, followRenamedPathInTabs } from '@entities/layout/tab-path-change'

/**
 * Refreshes the command palette's quick-open file index after an in-app filesystem mutation
 * (contract `2026-09-04-usability-batch3-contract.md` §A.2 item 5). `SEARCH.PROJECT_FILES` has no
 * backend cache behind it — it is one `search_list_files` walk frozen in the query cache — so any
 * create/rename/copy/delete this app performs makes it wrong until something invalidates it, and
 * until now the *only* invalidator was `ipc-sync-provider`'s debounced watcher echo.
 *
 * The invalidation is demand-driven, not immediate: the palette query is `enabled` only while the
 * palette is open, and `refetchQueries` skips every disabled query whatever `refetchType` says
 * (query-core 5.101 `queryClient.refetchQueries` filters on `!query.isDisabled()`, and
 * `isDisabled()` is `!isActive()` as soon as one observer exists — the always-mounted
 * `<CommandPalette/>` is that observer). So a mutation made with the palette closed marks the
 * index stale and the re-walk runs when the palette is next opened: still one walk per user file
 * operation, but the first frame of that ⌘P renders the stale array while the walk is in flight.
 * Two things cover that frame — the `palette.filesRefreshing` indicator on the files group, and
 * `layout_open_tab`'s existence pre-check, which refuses a row whose file is gone instead of
 * opening a broken tab. `refetchType: 'all'` only widens the invalidation to an index with no live
 * observer (a project that is no longer the active one); keeping the refetch demand-driven is also
 * what the watcher echo cannot do safely (an external bulk change fans out into many `fs:changed`
 * batches — hence that path deliberately stays on the default `refetchType`).
 *
 * `useSaveFile` is intentionally not on this list even though it writes to disk: a save targets a
 * path the index already carries, and auto-save makes its frequency unbounded, so invalidating
 * here would buy nothing and cost a project-wide walk per keystroke-idle. The one case that does
 * create a new file — an untitled tab's Save As — is covered by the watcher's `created` echo.
 */
const invalidateProjectFileIndex = (queryClient: QueryClient, projectId: ProjectId) =>
    queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId), refetchType: 'all' })

export const fileQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.CONTENT(path ?? ''),
        queryFn: () => openFile(path ?? ''),
        enabled: !!path,
        staleTime: Infinity,
    })

/**
 * Restorable hot-exit mirrors for the project's file tabs, fetched once when a project becomes
 * active (`staleTime: Infinity` — invalidated explicitly on save/clear/prune rather than refetched
 * on a timer) and shared by every editor pane through the query cache.
 */
export const fileMirrorsQueryOptions = (projectId: ProjectId) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.MIRRORS(projectId),
        queryFn: () => listMirrors(projectId),
        staleTime: Infinity,
    })

export const untitledMirrorsQueryOptions = (projectId: ProjectId) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId),
        queryFn: () => listUntitledMirrors(projectId),
        staleTime: Infinity,
    })

export const fileRawQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.RAW(path ?? ''),
        queryFn: () => readFileRaw(path ?? ''),
        enabled: !!path,
        staleTime: Infinity,
    })

/**
 * Saving a file changes its git status and settles its hot-exit mirror, so callers that know the
 * owning project pass `projectId` to have those scopes invalidated here (the entities layer owns
 * invalidation — see `useRenameEntry` for the same pattern) instead of each widget re-invalidating
 * by hand. Callers without a project context (`ide-sync-provider`) omit it and keep the
 * path-scoped `FILE.CONTENT` refresh only.
 */
export const useSaveFile = (projectId?: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: saveFile,
        /**
         * The cache patch below (before the invalidation) makes `FILE.CONTENT` reflect the content
         * this mutation just committed to disk *synchronously with* the success callback, instead of
         * only after the invalidation's refetch lands. Consumers treat that cache as "what is on
         * disk" the moment they observe a clean (non-dirty) editor — `editor-pane.tsx`'s render-body
         * adoption branch re-derives its `syncedContent` from it on every dirty→false transition, so
         * a still-stale cache at that instant gets re-adopted and applied over the just-saved buffer,
         * reverting the editor to its pre-save content and re-marking it dirty with no path back to
         * clean (docs/acknowledge/2026-08-27-d43-save-stale-sync-clobber-contract.md §0 — the race
         * reproduced whenever the refetch lost to the save's own settle re-render, which a
         * slow/remote round trip makes routine). `modifiedMs` intentionally stays stale here: only
         * the refetch knows the authoritative post-write mtime, and patching content alone is
         * already enough to make every adoption in the window a same-content no-op.
         *
         * `publishFileSaveSettle` is the path-scoped half of the same idea: the cache patch tells
         * every pane what is now on disk, but each `EditorPane` also keeps its own `dirty` /
         * `syncedContent` / hot-exit-mirror bookkeeping in React state, which only the pane that
         * issued the save used to reset. Announcing the write here — the one place every save flows
         * through, whether it came from ⌘S, auto-save, `ide-sync-provider`'s Claude Code save, or an
         * untitled tab's Save As — settles every pane on the path instead of just one.
         */
        onSuccess: (_, { path, content }) => {
            queryClient.setQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(path), (existing) => (existing ? { ...existing, content } : existing))
            publishFileSaveSettle(path, content)
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
            if (!projectId) return
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
        },
    })
}

export const useCreateEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: createEntry,
        onSuccess: () => void invalidateProjectFileIndex(queryClient, projectId),
    })
}

/**
 * `onSuccess` is `async` on purpose: TanStack Query keeps the mutation pending until the promise it
 * returns settles, so the explorer's rename flow (`use-explorer-entry-crud.ts` awaits
 * `renameEntryAsync`) only proceeds to its tree refresh/reveal once every open tab has been moved to
 * the new path — no window ever renders the renamed file under a path that no longer exists.
 *
 * `followRenamedPathInTabs` is wired here rather than in the explorer widget so the explorer's own
 * renames all reach it: the inline rename and the cut-and-paste move (`use-explorer-clipboard.ts`
 * runs a `rename` too) both funnel through this one mutation (audit §4-B A3). It is *not* the only
 * way a file gets renamed — `shared/lib/lsp/workspace-edit-applier.ts` calls `commands.fileRename`
 * directly for an LSP `RenameFile` operation and therefore still does not move tabs (carried in
 * contract §5 S8 / d-51 §5 F1).
 *
 * `onMutate` re-reads the project's mirror list *before* the rename lands, because that list is the
 * only place an unsaved draft belonging to **another OS window** can be read from: model registries
 * and query caches are per-window module instances, and `FILE.MIRRORS` is `staleTime: Infinity`, so
 * this window's copy is a project-activation snapshot that predates anything the other window has
 * typed. Refreshing it here (rather than after the rename) is what makes the draft reachable at all
 * — `list_mirrors` skips every mirror whose file no longer exists on disk, so once `from` is gone
 * its mirror can never be listed again, and the next `prune_mirrors` sweep deletes it.
 *
 * `onSuccess`'s failure is swallowed: the file is already renamed on disk at this point, so
 * rejecting here would report a successful rename as failed — and `use-explorer-clipboard.ts`'s
 * paste retry, which only re-runs on `error.file.destinationExists`, would surface it as a dead-end
 * error toast.
 */
export const useRenameEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: renameEntry,
        onMutate: async () => {
            await queryClient.fetchQuery({ ...fileMirrorsQueryOptions(projectId), staleTime: 0 }).catch(() => undefined)
        },
        onSuccess: async (_, { from, to }) => {
            const result = await followRenamedPathInTabs({ queryClient, projectId, from, to }).catch(() => null)
            if (result) applyFreshLayout(queryClient, projectId, result.layout)
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(from) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(to) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
            void invalidateProjectFileIndex(queryClient, projectId)
        },
    })
}

export const useCopyEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: copyEntry,
        onSuccess: (_, { to }) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(to) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
            void invalidateProjectFileIndex(queryClient, projectId)
        },
    })
}

/**
 * Closes the tabs the deleted path (or anything under it) was open in before the invalidations
 * below — a tab left pointing at a deleted file re-creates it on the next `⌘S`
 * (`write_atomic`'s `create_dir_all`), and its `FILE.CONTENT` refetch would surface as an error
 * banner in a pane the user never asked to keep. Async — and failure-swallowing — for the same
 * reasons as `useRenameEntry`.
 */
export const useDeleteEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: deleteEntry,
        onSuccess: async (_, path) => {
            const result = await followDeletedPathInTabs({ queryClient, projectId, path }).catch(() => null)
            if (result) applyFreshLayout(queryClient, projectId, result.layout)
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
            void invalidateProjectFileIndex(queryClient, projectId)
        },
    })
}
