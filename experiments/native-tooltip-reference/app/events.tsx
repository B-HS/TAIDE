import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import type { FC } from 'react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@shared/ui/tooltip'
import '@shared/styles/global.css'

const TRIGGERS = [
    { id: 'first', left: 80 },
    { id: 'second', left: 240 },
] as const
const TOP = 100
const INPUT_TOP = 200
const WIDTH = 40
const HEIGHT = 20
const INPUT_WIDTH = 120
const DELAY_MS = 400
const HAS_CONTROLLED_INPUT = new URL(window.location.href).searchParams.get('mode') === 'controlled'

export const TooltipEventReference: FC = () => {
    const [hasInput, setHasInput] = useState(HAS_CONTROLLED_INPUT)
    const [records, setRecords] = useState<string[]>([])
    const handleRecord = (owner: string, isOpen: boolean) => setRecords((previous) => [...previous, `${owner}:${isOpen ? 'open' : 'closed'}`])
    const handleBlur = () => {
        handleRecord('input-blur', false)
        setHasInput(false)
    }
    return (
        <TooltipProvider delayDuration={DELAY_MS}>
            <output data-records>{records.join('|')}</output>
            {TRIGGERS.map(({ id, left }) => (
                <Tooltip key={id} onOpenChange={(isOpen) => handleRecord(id, isOpen)}>
                    <TooltipTrigger asChild>
                        <button type='button' data-event-trigger={id} style={{ position: 'fixed', left, top: TOP, width: WIDTH, height: HEIGHT }}>
                            {id}
                        </button>
                    </TooltipTrigger>
                    <TooltipContent data-event-content={id}>Tooltip {id}</TooltipContent>
                </Tooltip>
            ))}
            {hasInput && (
                <Tooltip open onOpenChange={(isOpen) => handleRecord('controlled', isOpen)}>
                    <TooltipTrigger asChild>
                        <input
                            autoFocus
                            aria-label='Synthetic draft'
                            aria-invalid
                            data-event-input
                            onBlur={handleBlur}
                            style={{ position: 'fixed', left: TRIGGERS[1].left, top: INPUT_TOP, width: INPUT_WIDTH, height: HEIGHT }}
                        />
                    </TooltipTrigger>
                    <TooltipContent side='bottom' data-event-content='controlled'>
                        Validation error
                    </TooltipContent>
                </Tooltip>
            )}
        </TooltipProvider>
    )
}

const root = document.getElementById('root')
if (!root) throw new Error('Missing synthetic tooltip root')
createRoot(root).render(<TooltipEventReference />)
