import { Channel } from '@tauri-apps/api/core'
import { commands } from '@shared/api/bindings'
import type { ProjectId, SearchMatch, SearchQuery } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const runSearch = (input: { projectId: ProjectId; query: SearchQuery; onMatch: (match: SearchMatch) => void }) => {
    const channel = new Channel<SearchMatch>()
    channel.onmessage = (match) => input.onMatch(match)
    return unwrapResult(commands.searchRun(input.projectId, input.query, channel))
}

export const cancelSearch = (projectId: ProjectId) => unwrapResult(commands.searchCancel(projectId))

export const replaceSearch = (input: { projectId: ProjectId; query: SearchQuery; replacement: string; paths: string[] | null }) =>
    unwrapResult(commands.searchReplace(input.projectId, input.query, input.replacement, input.paths))
