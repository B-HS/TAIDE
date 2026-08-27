import { queryOptions, useMutation } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { listProjectFiles, replaceSearch } from '@entities/search/search.ipc'

/**
 * Wraps `replaceSearch`'s request/response call in `useMutation` instead of a widget hand-rolling
 * its own `.then`/`.catch`/`.finally` chain and `isReplacing` state (contract F4#5) — `isPending`
 * already tracks that, so the caller no longer needs a parallel `useState` for it.
 */
export const useReplaceSearch = () => useMutation({ mutationFn: replaceSearch })

/**
 * The command palette's file quick-open index (contract `2026-08-25-d42-e2e-defects-contract.md`
 * §3, item d) — a full project-wide file listing, independent of the Explorer tree's lazy-loaded
 * `TREE.ROWS` cache. `command-palette.tsx` fuzzy-filters the result client-side per keystroke
 * (mirroring how it already treated `treeRowsQueryOptions`'s page), so this only needs to fetch
 * once per palette-open — the default global `staleTime` (`app/query-client.ts`) is enough, and
 * `IpcSyncProvider`'s `fs:changed` handler invalidates this alongside `TREE.ROWS` for any
 * create/rename/delete so it doesn't go stale for the lifetime of an open project.
 *
 * `projectId: ProjectId | null` + built-in `enabled: !!projectId` mirrors `tree.query.ts`'s
 * `treeRowsQueryOptions` — the caller passes whatever `activeProjectId` it already has, no `?? ''`
 * placeholder, and this factory alone decides when the query may run rather than trusting every
 * call site to remember its own `enabled` guard.
 */
export const projectFilesQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId ?? ''),
        queryFn: () => listProjectFiles(projectId ?? ''),
        enabled: !!projectId,
    })
