import { useLayoutEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { AlertTriangle } from 'lucide-react'
import { currentLocaleQueryOptions } from '@entities/locale/locale.query'
import { applyLocaleMessages } from '@shared/i18n/i18n'

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
                <div className='bg-status-error/15 text-status-error fixed inset-x-0 top-0 z-50 flex items-center gap-2 px-3 py-1.5 text-xs'>
                    <AlertTriangle className='size-3.5 shrink-0' />
                    <span className='flex-1'>{LOCALE_LOAD_ERROR_FALLBACK_MESSAGE}</span>
                    <button type='button' onClick={() => void refetch()} className='underline'>
                        {t('common.retry')}
                    </button>
                </div>
            )}
            {children}
        </>
    )
}
