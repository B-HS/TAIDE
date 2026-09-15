import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { SessionShellState } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { closeShellSlot, focusShellSlot, getShellState, openProjectInSlot, setShellSlotSizes, setWindowChrome } from '@entities/session/session.ipc'

/**
 * The window's shell-slot arrangement plus the window-level chrome axes (contract §0.1 S-6). Read
 * once per window at mount and refreshed from `session:shell-slots-changed` /
 * `session:window-chrome-changed` (`ipc-sync-provider.tsx`) — both events fire only at transitions,
 * which is why the command exists at all.
 */
export const shellStateQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.SESSION.SHELL_STATE, queryFn: getShellState })

/** No invalidation of its own: Rust answers a focus move with `session:shell-slots-changed`, and the view already tracks focus locally (contract §0.1 U-7) so it never waits for this round trip. */
export const useFocusShellSlot = () => useMutation({ mutationFn: focusShellSlot })

export const useSetShellSlotSizes = () =>
    useMutation({ mutationFn: ({ path, sizes }: { path: number[]; sizes: number[] }) => setShellSlotSizes(path, sizes) })

export const useCloseShellSlot = () => useMutation({ mutationFn: closeShellSlot })

/**
 * Writes the answer straight into the cache rather than waiting for the event echo — the slot the
 * drop created has to exist before the next paint, or the new project flashes in the old
 * arrangement first. `PROJECT.ALL` still has to be invalidated because opening by path adds a
 * project the sidebar does not know about yet.
 */
export const useOpenProjectInSlot = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: openProjectInSlot,
        onSuccess: (state) => {
            queryClient.setQueryData(QUERY_KEY.SESSION.SHELL_STATE, state)
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
        },
    })
}

export const useSetWindowChrome = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: setWindowChrome,
        onSuccess: (windowChrome) =>
            queryClient.setQueryData<SessionShellState>(QUERY_KEY.SESSION.SHELL_STATE, (state) => (state ? { ...state, windowChrome } : state)),
    })
}
