import { createRoot } from 'react-dom/client'
import type { FC } from 'react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@shared/ui/tooltip'
import '@shared/styles/global.css'

const BUTTON_SIZE = 24
const DELAY_MS = 400
const CASES = [
    { id: 'top', side: 'top', left: 200, top: 100 },
    { id: 'bottom', side: 'bottom', left: 400, top: 100 },
    { id: 'left', side: 'left', left: 200, top: 220 },
    { id: 'right', side: 'right', left: 400, top: 220 },
    { id: 'shift-left', side: 'bottom', left: 0, top: 340 },
    { id: 'shift-right', side: 'bottom', left: 616, top: 340 },
    { id: 'flip-top', side: 'top', left: 200, top: 0 },
    { id: 'flip-bottom', side: 'bottom', left: 400, top: 456 },
] as const

export const TooltipReference: FC = () => (
    <TooltipProvider delayDuration={DELAY_MS}>
        {CASES.map((sample) => (
            <Tooltip key={sample.id} open>
                <TooltipTrigger asChild>
                    <button
                        type='button'
                        data-sample={sample.id}
                        aria-label={sample.id}
                        style={{ position: 'fixed', left: sample.left, top: sample.top, width: BUTTON_SIZE, height: BUTTON_SIZE }}>
                        {sample.id}
                    </button>
                </TooltipTrigger>
                <TooltipContent data-sample-content={sample.id} side={sample.side}>
                    Tooltip reference
                </TooltipContent>
            </Tooltip>
        ))}
    </TooltipProvider>
)

const root = document.getElementById('root')
if (!root) throw new Error('Missing synthetic tooltip root')
createRoot(root).render(<TooltipReference />)
