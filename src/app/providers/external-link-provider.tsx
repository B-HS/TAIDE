import { useEffect, type FC, type PropsWithChildren } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { openExternalUrl } from '@entities/system/external-url'
import { shouldOpenAnchorExternally } from '@shared/lib/external-anchor'

const PRIMARY_MOUSE_BUTTON = 0

/**
 * Catches every left-click on an anchor that points off the app's own origin and routes it to
 * {@link openExternalUrl} instead of letting the webview navigate itself there.
 *
 * This is the app-wide half of "external URLs always leave the app": the terminal owns its own
 * link activation, but plain `<a href>` markup — rendered markdown above all — reaches the DOM
 * from any widget, and a webview navigation to a foreign origin replaces the whole app UI with a
 * page that has no address bar or back button to escape from. A capture-phase listener on
 * `document` is the one place that sees those clicks regardless of which widget rendered them,
 * and `composedPath()` finds the anchor even when the click landed on an element nested inside
 * it (or inside a shadow root). Clicks a component already handled (`defaultPrevented`) and
 * non-primary buttons are left alone, so context menus and existing in-app handlers keep working.
 *
 * Mounted in both window branches of `app.tsx` — an auxiliary editor window renders the same
 * markdown preview as the main one and would strand itself the same way.
 */
export const ExternalLinkProvider: FC<PropsWithChildren> = ({ children }) => {
    const { t } = useTranslation()

    useEffect(() => {
        const handleClick = (event: MouseEvent) => {
            if (event.button !== PRIMARY_MOUSE_BUTTON || event.defaultPrevented) return
            const anchor = event.composedPath().find((target): target is HTMLAnchorElement => target instanceof HTMLAnchorElement)
            if (!anchor || !shouldOpenAnchorExternally(anchor.href, window.location.origin)) return
            event.preventDefault()
            void openExternalUrl(anchor.href).catch(() => toast.error(t('common.openExternalLinkFailed')))
        }

        document.addEventListener('click', handleClick, true)
        return () => document.removeEventListener('click', handleClick, true)
    }, [t])

    return children
}
