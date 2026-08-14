import type { FC, PropsWithChildren } from 'react'
import { events } from '@shared/api/bindings'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { flushMirrorsComplete } from '@entities/file/file.ipc'
import { flushAllMirrors } from '@entities/editor/mirror-flush-registry'

/**
 * Answers the Rust `CloseRequested` interception: the backend defers the actual window close and
 * emits `HotExitFlushRequested`, this provider pushes every currently mounted editor pane's
 * pending hot-exit mirror write (registered via `mirror-flush-registry`) to disk, then confirms
 * completion so the app can resume exiting. A backend-side timeout force-closes the window if this
 * never reports back, so a swallowed flush failure here can't hang the app.
 */
export const HotExitFlushProvider: FC<PropsWithChildren> = ({ children }) => {
    useTauriEvent(events.appHotExitFlushRequested, () => {
        void flushAllMirrors().finally(() => void flushMirrorsComplete().catch(() => undefined))
    })

    return children
}
