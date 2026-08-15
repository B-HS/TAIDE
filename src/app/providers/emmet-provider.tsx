import { useEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery } from '@tanstack/react-query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { enableEmmet } from '@shared/lib/emmet-integration'
import { monaco } from '@shared/lib/monaco/setup'

/**
 * Mounts/tears down Emmet abbreviation expansion (`shared/lib/emmet-integration.ts`) as an
 * external-system sync on the `settings.emmetEnabled` toggle (contract §3.4, default `true`) — an
 * `useEffect` keyed on the resolved boolean is the only way to dispose the library's monaco
 * registrations and re-register them, since `enableEmmet` has no partial on/off switch of its own.
 */
export const EmmetProvider: FC<PropsWithChildren> = ({ children }) => {
    const { data: settings } = useQuery(settingsQueryOptions())
    const emmetEnabled = settings?.emmetEnabled ?? true

    useEffect(() => {
        if (!emmetEnabled) return
        return enableEmmet(monaco)
    }, [emmetEnabled])

    return children
}
