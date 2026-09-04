import { getCurrentWindow } from '@tauri-apps/api/window'
import { REMOTE_WINDOW_LABEL } from '@shared/lib/remote/tauri-internals-shim'

/**
 * Pure half of {@link isRemoteMirrorRuntime}, split out so it can run under `bun:test` — that
 * runtime has no `window` global at all, and `getCurrentWindow()` reads its label straight out of
 * `window.__TAURI_INTERNALS__.metadata` (the same constraint `window-context.ts` documents for
 * `location.search`).
 */
export const isRemoteMirrorLabel = (label: string) => label === REMOTE_WINDOW_LABEL

/**
 * Whether this JS realm is the remote-mirror browser client rather than a desktop webview.
 *
 * `installRemoteInternalsShim` is the only thing that ever reports {@link REMOTE_WINDOW_LABEL}
 * through `getCurrentWindow().label` — every real desktop window carries `main` or `editor-<n>`
 * (`tauri.conf.json`, `domain::window::commands::open_auxiliary_window`) — so the label the shim
 * already publishes is a sufficient signal and no new global state is needed. Consumers use this
 * to pick *how* to leave the app: a desktop webview must hand the URL to the OS through IPC,
 * while the mirror runs in a real browser where `window.open` is the only path available (the
 * remote dispatcher rejects `system_open_external_url` outright — `remote/dispatch.rs`).
 */
export const isRemoteMirrorRuntime = () => typeof window !== 'undefined' && isRemoteMirrorLabel(getCurrentWindow().label)
