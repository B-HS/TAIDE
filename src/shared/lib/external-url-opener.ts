export type ExternalUrlOpenerDeps = {
    isRemoteMirror: () => boolean
    openViaShell: (uri: string) => Promise<unknown>
    openViaBrowser: (uri: string) => Window | null
}

/**
 * Builds the app's single "send this URL out of the app" function.
 *
 * The two runtimes get *disjoint* paths rather than a fallback chain, which is the whole point of
 * this factory. On the desktop the URL always goes to the OS through `system_open_external_url`
 * IPC and `window.open` is never called at all: a webview that does open a popup would put a
 * foreign page inside a TAIDE window with no chrome to leave it, and a webview that refuses one
 * used to make the click look dead while the IPC path was still available. In the remote mirror
 * the reverse holds — the page runs in a real browser, and the remote dispatcher rejects
 * `system_open_external_url` by design (`remote/dispatch.rs`), so a new tab is the only exit.
 * A mirror whose browser refused the popup (blocker, sandboxed frame) throws so the caller can
 * surface it, since there is nothing left to try.
 */
export const createExternalUrlOpener = (deps: ExternalUrlOpenerDeps) => async (uri: string) => {
    if (!deps.isRemoteMirror()) {
        await deps.openViaShell(uri)
        return
    }
    if (deps.openViaBrowser(uri)) return
    throw new Error(`browser refused to open ${uri}`)
}

type NativeWindowOpen = () => Window | null

/**
 * Strips the freshly opened tab's back-reference to this window. Reports whether it worked
 * rather than throwing, because some engines (Electron among them) refuse the reassignment
 * outright — the tab is open either way, so the caller still navigates it.
 */
const revokeOpener = (target: Window) => {
    try {
        target.opener = null
        return true
    } catch {
        return false
    }
}

/**
 * Opens `uri` in a new browser tab without ever passing `noopener` to `window.open`.
 * Per the HTML window-open steps, `window.open(url, target, 'noopener')` always returns
 * `null` even when the tab opened successfully, so a caller branching on that return
 * value can never distinguish a real rejection from a success. Instead this opens a blank
 * tab first, then manually revokes `opener` and navigates it to `uri` — the same two-step
 * pattern `@xterm/addon-web-links`'s own default handler uses to sidestep the same pitfall.
 * Returns the opened window, or `null` when the environment refused to open one at all.
 * Only the remote mirror calls this — see {@link createExternalUrlOpener}.
 */
export const openViaBrowserWindow = (uri: string, nativeOpen: NativeWindowOpen): Window | null => {
    const opened = nativeOpen()
    if (!opened) return null
    revokeOpener(opened)
    opened.location.href = uri
    return opened
}
