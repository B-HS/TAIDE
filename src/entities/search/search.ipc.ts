import { Channel } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { commands } from '@shared/api/bindings'
import type { ProjectId, SearchMatch, SearchQuery } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

/**
 * `owner` (this window's OS label) scopes the search session to the calling window — mirrors
 * `entities/lsp/lsp.ipc.ts`'s `spawnLspSession`/`lsp_stop` `owner` precedent. Needed because
 * `sessionId` alone (`useId()` in `search-panel-container.tsx`, or a Search Editor tab id) is only
 * unique *within* one window's own React realm: a second window can independently mint the exact
 * same id, which — before the backend's `(owner, session_id)` composite key (R7#8) — let one
 * window's `searchCancel` truncate a different window's in-flight search.
 */
export const runSearch = (input: { projectId: ProjectId; sessionId: string; query: SearchQuery; onMatch: (match: SearchMatch) => void }) => {
    const channel = new Channel<SearchMatch>()
    channel.onmessage = (match) => input.onMatch(match)
    return unwrapResult(commands.searchRun(input.projectId, getCurrentWindow().label, input.sessionId, input.query, channel))
}

export const cancelSearch = (sessionId: string) => unwrapResult(commands.searchCancel(getCurrentWindow().label, sessionId))

export const replaceSearch = (input: { projectId: ProjectId; query: SearchQuery; replacement: string; paths: string[] | null }) =>
    unwrapResult(commands.searchReplace(input.projectId, input.query, input.replacement, input.paths))
