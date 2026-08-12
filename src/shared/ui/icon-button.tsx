import type { ComponentProps, FC, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

export type IconButtonSide = 'top' | 'right' | 'bottom' | 'left'

type IconButtonProps = Omit<ComponentProps<'button'>, 'aria-label' | 'children'> & {
    label: string
    icon: ReactNode
    side?: IconButtonSide
}

export const IconButton: FC<IconButtonProps> = ({ label, icon, side = 'bottom', type = 'button', disabled, onClick, className, ...props }) => (
    <Tooltip>
        <TooltipTrigger asChild>
            <span className='inline-flex' tabIndex={disabled ? 0 : undefined}>
                <button
                    type={type}
                    aria-label={label}
                    aria-disabled={disabled || undefined}
                    disabled={disabled}
                    onClick={disabled ? undefined : onClick}
                    className={cn(disabled && 'pointer-events-none', className)}
                    {...props}>
                    {icon}
                </button>
            </span>
        </TooltipTrigger>
        <TooltipContent side={side}>{label}</TooltipContent>
    </Tooltip>
)
