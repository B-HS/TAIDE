import type { ProjectId, TabId } from '@shared/api/bindings'

const contentByProjectId = new Map<ProjectId, Map<TabId, string>>()

export const getUntitledContent = (projectId: ProjectId, tabId: TabId) => contentByProjectId.get(projectId)?.get(tabId) ?? null

export const setUntitledContent = (projectId: ProjectId, tabId: TabId, content: string) => {
    const contentByTabId = contentByProjectId.get(projectId) ?? new Map<TabId, string>()
    contentByTabId.set(tabId, content)
    contentByProjectId.set(projectId, contentByTabId)
}

export const dropUntitledContent = (projectId: ProjectId, tabId: TabId) => {
    contentByProjectId.get(projectId)?.delete(tabId)
}

export const pruneUntitledContents = (projectId: ProjectId, keepTabIds: TabId[]) => {
    const contentByTabId = contentByProjectId.get(projectId)
    if (!contentByTabId) return []

    const keep = new Set(keepTabIds)
    const removed = [...contentByTabId.keys()].filter((tabId) => !keep.has(tabId))
    for (const tabId of removed) contentByTabId.delete(tabId)
    return removed
}
