import { useLayoutEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery } from '@tanstack/react-query'
import { currentLocaleQueryOptions } from '@entities/locale/locale.query'
import { applyLocaleMessages } from '@shared/i18n/i18n'

export const LocaleProvider: FC<PropsWithChildren> = ({ children }) => {
    const { data: locale } = useQuery(currentLocaleQueryOptions())

    useLayoutEffect(() => {
        if (!locale) return
        applyLocaleMessages(locale.id, locale.messages)
        document.documentElement.lang = locale.id
    }, [locale])

    return children
}
