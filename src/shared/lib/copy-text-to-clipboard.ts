import { toast } from 'sonner'
import { i18next } from '@shared/i18n/i18n'

/**
 * Writes `text` to the system clipboard and reports whether it landed.
 *
 * The context menus that copy a path used to call `navigator.clipboard.writeText` bare, which fails
 * two different ways that both ended up invisible. In the desktop webview the write can be refused
 * outright ("The request is not allowed by the user agent or the platform in the current context"),
 * and the rejected promise — dropped with `void` — surfaced only as an ERROR line in the file log
 * through the global `unhandledrejection` forwarder (`error-log-forwarding.ts`), never to the user.
 * In the remote mirror, served over plain LAN HTTP, `navigator.clipboard` is absent altogether
 * (`terminal-clipboard-availability.ts`), so the very same expression throws *synchronously*: a
 * `.catch()` chained onto the call is not even evaluated in that case, which is why the guard here
 * is a `try`/`catch` around the call rather than a rejection handler on its result (the reasoning
 * `monaco/on-save-cleanup.ts` spells out for monaco's nominally-async `IEditorAction.run`).
 *
 * Failure is announced here because a copy that quietly did nothing is indistinguishable from one
 * that worked. Success is not: a path copy closes its menu on its own and a toast per copy would be
 * noise, so the few call sites that do want confirmation raise it themselves off the returned flag.
 */
export const copyTextToClipboard = async (text: string) => {
    try {
        await navigator.clipboard.writeText(text)
        return true
    } catch {
        toast.error(i18next.t('common.copyFailed'))
        return false
    }
}
