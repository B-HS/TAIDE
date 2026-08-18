import { useLayoutEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { currentLocaleQueryOptions } from '@entities/locale/locale.query'
import { applyLocaleMessages } from '@shared/i18n/i18n'
import { StatusErrorBanner } from '@shared/ui/status-error-banner'

const LOCALE_LOAD_ERROR_FALLBACK_MESSAGE = 'Failed to load the app language pack. Some labels may show as raw keys.'

export const LocaleProvider: FC<PropsWithChildren> = ({ children }) => {
    const { data: locale, isFetched, isError, refetch } = useQuery(currentLocaleQueryOptions())
    const { t } = useTranslation()

    useLayoutEffect(() => {
        if (!locale) return
        applyLocaleMessages(locale.id, locale.messages)
        document.documentElement.lang = locale.id
    }, [locale])

    useLayoutEffect(() => {
        if (!isFetched) return
        document.documentElement.dataset.localeReady = ''
    }, [isFetched])

    return (
        <>
            {isError && (
                <StatusErrorBanner message={LOCALE_LOAD_ERROR_FALLBACK_MESSAGE} retryLabel={t('common.retry')} onRetry={() => void refetch()} />
            )}
            {children}
        </>
    )
}
