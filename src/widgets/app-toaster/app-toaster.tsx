import { useQuery } from '@tanstack/react-query'
import { Toaster } from 'sonner'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { DEFAULT_TOAST_POSITION, isMiddleToastPosition, MIDDLE_TOAST_CLASS, toSonnerPosition } from '@shared/constants/toast'

export const AppToaster = () => {
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: theme } = useQuery(currentThemeQueryOptions())

    const position = settings?.toastPosition ?? DEFAULT_TOAST_POSITION

    return (
        <Toaster
            theme={theme?.type ?? 'dark'}
            position={toSonnerPosition(position)}
            className={isMiddleToastPosition(position) ? MIDDLE_TOAST_CLASS : undefined}
            richColors
            closeButton
        />
    )
}
