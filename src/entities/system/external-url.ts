import { systemOpenExternalUrl } from '@entities/system/system.ipc'
import { createExternalUrlOpener, openViaBrowserWindow } from '@shared/lib/external-url-opener'
import { isRemoteMirrorRuntime } from '@shared/lib/remote/runtime-environment'

/**
 * The app's one and only way to send a URL outside TAIDE — terminal links (both the plain-text
 * matcher and OSC 8 hyperlinks), rendered-markdown anchors, and anything added later. Keeping a
 * single binding here is what makes "no external page ever renders inside an app window" a
 * property of the app rather than of each call site: the desktop branch goes through
 * `system_open_external_url`, whose Rust side re-validates the scheme against an http(s)
 * whitelist before handing it to the OS.
 */
export const openExternalUrl = createExternalUrlOpener({
    isRemoteMirror: isRemoteMirrorRuntime,
    openViaShell: systemOpenExternalUrl,
    openViaBrowser: (uri) => openViaBrowserWindow(uri, () => window.open()),
})
