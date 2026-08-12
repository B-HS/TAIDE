import type { FC } from 'react'
import { cn } from '@shared/lib/cn'
import { TOAST_HORIZONTAL_POSITIONS, TOAST_VERTICAL_POSITIONS } from '@shared/constants/toast'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const POSITION_LABEL_KEY: Record<string, string> = {
    'top-left': 'settings.positionTopLeft',
    'top-center': 'settings.positionTopCenter',
    'top-right': 'settings.positionTopRight',
    'middle-left': 'settings.positionMiddleLeft',
    'middle-center': 'settings.positionMiddleCenter',
    'middle-right': 'settings.positionMiddleRight',
    'bottom-left': 'settings.positionBottomLeft',
    'bottom-center': 'settings.positionBottomCenter',
    'bottom-right': 'settings.positionBottomRight',
}

type ToastPositionPickerProps = {
    value: string
    translateLabel: (key: string) => string
    onSelect: (position: string) => void
}

export const ToastPositionPicker: FC<ToastPositionPickerProps> = ({ value, translateLabel, onSelect }) => (
    <div className='grid w-40 grid-cols-3 gap-1'>
        {TOAST_VERTICAL_POSITIONS.flatMap((vertical) =>
            TOAST_HORIZONTAL_POSITIONS.map((horizontal) => {
                const position = `${vertical}-${horizontal}`
                const isActive = position === value
                const label = translateLabel(POSITION_LABEL_KEY[position] ?? '')
                return (
                    <Tooltip key={position}>
                        <TooltipTrigger asChild>
                            <button
                                type='button'
                                onClick={() => onSelect(position)}
                                aria-pressed={isActive}
                                aria-label={label}
                                className={cn(
                                    'border-app-border hover:bg-app-sidebar-item-hover h-9 rounded-md border',
                                    isActive && 'border-app-focus-border bg-app-accent/20',
                                )}
                            />
                        </TooltipTrigger>
                        <TooltipContent side='top'>{label}</TooltipContent>
                    </Tooltip>
                )
            }),
        )}
    </div>
)
