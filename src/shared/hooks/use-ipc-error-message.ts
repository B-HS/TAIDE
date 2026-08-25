import { useTranslation } from 'react-i18next'
import { i18next } from '@shared/i18n/i18n'
import { hasLocaleKey } from '@shared/lib/ipc-error-message'

/**
 * `describeIpcError` (`@shared/lib/ipc-error-message`) for text that stays on screen after the
 * triggering event (unlike a toast, which renders once and is gone) — reads `t` from
 * {@link useTranslation} so the text re-resolves if the viewer switches language while it's still
 * visible, the same reactivity every other piece of persistent UI text in this codebase gets for
 * free from `t(...)`.
 */
export const useIpcErrorMessage = (error: unknown) => {
    const { t } = useTranslation()
    if (hasLocaleKey(error) && error.localeKey && i18next.exists(error.localeKey)) return t(error.localeKey, error.localeArgs ?? {})
    return error instanceof Error ? error.message : String(error)
}
