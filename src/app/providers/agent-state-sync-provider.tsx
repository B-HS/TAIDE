import type { FC, PropsWithChildren } from 'react'
import { useAgentStateSync } from '@entities/agent/agent.query'

/**
 * Keeps the `agent:state-changed` push sync (`useAgentStateSync`) alive independent of any
 * particular piece of chrome being mounted — it used to live inside `AppSidebar`, which unmounts
 * whenever Zen mode hides the sidebar (`app-shell.tsx`), silently dropping agent status updates for
 * as long as the user stayed in that view even though the agent badge in `PaneTabBar` (editor tab
 * bar, visible in both Zen and auxiliary windows) still reads the same `AGENT.PROJECT` cache entry.
 * Mounted once at the app root (`app.tsx`), alongside the other provider-layer IPC sync, for both
 * the main window and every auxiliary editor window — `payload.projectId` scopes each push, so
 * unlike `IdeSyncProvider` there is no risk of two windows racing to handle the same broadcast.
 */
export const AgentStateSyncProvider: FC<PropsWithChildren> = ({ children }) => {
    useAgentStateSync()
    return children
}
