import { useEffect, type PropsWithChildren, type FC } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { bindQueryClientToWindow, queryClient } from '@app/query-client'
import { TooltipProvider } from '@shared/ui/tooltip'
import { AppToaster } from '@widgets/app-toaster/app-toaster'

const TOOLTIP_DELAY_MS = 400

/**
 * `AppToaster` renders *inside* `TooltipProvider`: this project's `Tooltip` wrapper does not carry a
 * provider of its own, so the first toast to hold an `IconButton` (or anything else with a tooltip)
 * would throw out of Radix — and the toaster is mounted outside every `ErrorBoundary`, which would
 * make that throw take the whole window down instead of one panel.
 */
export const AppProviders: FC<PropsWithChildren> = ({ children }) => {
    useEffect(() => bindQueryClientToWindow(), [])

    return (
        <QueryClientProvider client={queryClient}>
            <TooltipProvider delayDuration={TOOLTIP_DELAY_MS}>
                {children}
                <AppToaster />
            </TooltipProvider>
        </QueryClientProvider>
    )
}
