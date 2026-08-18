export type TerminalLinkOpenerDeps = {
    windowOpen: (uri: string) => Window | null
    openExternalUrl: (uri: string) => Promise<unknown>
}

export const openTerminalLink = async (uri: string, deps: TerminalLinkOpenerDeps) => {
    if (deps.windowOpen(uri)) return
    await deps.openExternalUrl(uri)
}

type NativeWindowOpen = () => Window | null

/**
 * Opens `uri` in a new browser tab without ever passing `noopener` to `window.open`.
 * Per the HTML window-open steps, `window.open(url, target, 'noopener')` always returns
 * `null` even when the tab opened successfully, so a caller branching on that return
 * value can never distinguish a real desktop-webview rejection from a remote-mirror
 * success. Instead this opens a blank tab first, then manually revokes `opener` and
 * navigates it to `uri` — the same two-step pattern `@xterm/addon-web-links`'s own
 * default handler uses to sidestep the same pitfall. Returns the opened window, or
 * `null` when the environment refused to open one at all (e.g. the desktop webview),
 * so callers can fall back to an IPC-based opener.
 */
export const openViaBrowserWindow = (uri: string, nativeOpen: NativeWindowOpen): Window | null => {
    const opened = nativeOpen()
    if (!opened) return null
    try {
        opened.opener = null
    } catch {
        // Some engines refuse to reassign `opener`; the tab still opened, so proceed.
    }
    opened.location.href = uri
    return opened
}
