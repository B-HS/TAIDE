import { Channel } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { commands } from '@shared/api/bindings'
import type { ProjectId, SearchFileMatches, SearchQuery } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

/**
 * `owner` (this window's OS label) scopes the search session to the calling window — mirrors
 * `entities/lsp/lsp.ipc.ts`'s `spawnLspSession`/`lsp_stop` `owner` precedent. Needed because
 * `sessionId` alone (`useId()` in `search-panel-container.tsx`, or a Search Editor tab id) is only
 * unique *within* one window's own React realm: a second window can independently mint the exact
 * same id, which — before the backend's `(owner, session_id)` composite key (R7#8) — let one
 * window's `searchCancel` truncate a different window's in-flight search.
 *
 * The channel carries one message per *file* (`SearchFileMatches`), not per match: matches inside
 * a file arrive in ascending source order, but the order files arrive in is unspecified because
 * the backend walk is parallel.
 */
export const runSearch = (input: {
    projectId: ProjectId
    sessionId: string
    query: SearchQuery
    onFileMatches: (batch: SearchFileMatches) => void
}) => {
    const channel = new Channel<SearchFileMatches>()
    channel.onmessage = (batch) => input.onFileMatches(batch)
    return unwrapResult(commands.searchRun(input.projectId, getCurrentWindow().label, input.sessionId, input.query, channel))
}

export const cancelSearch = (sessionId: string) => unwrapResult(commands.searchCancel(getCurrentWindow().label, sessionId))

export const replaceSearch = (input: { projectId: ProjectId; query: SearchQuery; replacement: string; paths: string[] | null }) =>
    unwrapResult(commands.searchReplace(input.projectId, input.query, input.replacement, input.paths))

/**
 * Every file path under `projectId`'s root, as absolute paths — the command palette's file
 * quick-open index (contract `2026-08-25-d42-e2e-defects-contract.md` §3, item d). Unlike
 * `entities/tree/tree.ipc.ts`'s `getTreeRows` (only ever holds entries for a directory some caller
 * already `tree_toggle`'d open), this walks the whole project every call — see
 * `search_list_files`'s own doc comment for why quick-open needs that instead of depending on the
 * Explorer tree's lazy loading.
 */
export const listProjectFiles = (projectId: ProjectId) => unwrapResult(commands.searchListFiles(projectId))
