import { useEffect, type PropsWithChildren, type FC } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { Toaster } from 'sonner'
import { bindQueryClientToWindow, queryClient } from '@app/query-client'
import { TooltipProvider } from '@shared/ui/tooltip'

const TOOLTIP_DELAY_MS = 400

export const AppProviders: FC<PropsWithChildren> = ({ children }) => {
    useEffect(() => bindQueryClientToWindow(), [])

    return (
        <QueryClientProvider client={queryClient}>
            <TooltipProvider delayDuration={TOOLTIP_DELAY_MS}>{children}</TooltipProvider>
            <Toaster theme='dark' position='bottom-right' richColors closeButton />
        </QueryClientProvider>
    )
}
