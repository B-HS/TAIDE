import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import type { FC } from 'react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@shared/ui/tooltip'
import '@shared/styles/global.css'

const SIDES = ['top', 'bottom', 'left', 'right'] as const
const SIDE = SIDES.find((side) => side === new URL(window.location.href).searchParams.get('side')) ?? 'top'
const BUTTON_SIZE = 24
const LEFT = 200
const TOP = 100
const DELAY_MS = 400

export const TooltipMotionReference: FC = () => {
    const [isOpen, setIsOpen] = useState(true)
    return (
        <TooltipProvider delayDuration={DELAY_MS}>
            <button type='button' data-close onClick={() => setIsOpen(false)}>
                Close
            </button>
            <button type='button' data-open onClick={() => setIsOpen(true)}>
                Open
            </button>
            <Tooltip open={isOpen} onOpenChange={setIsOpen}>
                <TooltipTrigger asChild>
                    <button
                        type='button'
                        data-trigger
                        aria-label='trigger'
                        style={{ position: 'fixed', left: LEFT, top: TOP, width: BUTTON_SIZE, height: BUTTON_SIZE }}>
                        Trigger
                    </button>
                </TooltipTrigger>
                <TooltipContent data-motion-content side={SIDE}>
                    Tooltip reference
                </TooltipContent>
            </Tooltip>
        </TooltipProvider>
    )
}

const root = document.getElementById('root')
if (!root) throw new Error('Missing synthetic tooltip root')
createRoot(root).render(<TooltipMotionReference />)
