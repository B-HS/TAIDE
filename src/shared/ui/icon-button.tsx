import type { ComponentProps, FC, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

export type IconButtonSide = 'top' | 'right' | 'bottom' | 'left'

type IconButtonProps = Omit<ComponentProps<'button'>, 'aria-label' | 'children'> & {
    label: string
    icon: ReactNode
    side?: IconButtonSide
    containerClassName?: string
}

/**
 * The tooltip trigger wraps the button in a span so a disabled button still shows its tooltip
 * (pointer events land on the span). That span — not the button — is the flex/grid child, so
 * layout-positioning utilities (mt-auto, ml-auto, hidden/group-hover, ...) must go into
 * `containerClassName`; `className` styles the button itself (colors, size, disabled variants).
 */
export const IconButton: FC<IconButtonProps> = ({
    label,
    icon,
    side = 'bottom',
    type = 'button',
    disabled,
    onClick,
    className,
    containerClassName,
    ...props
}) => (
    <Tooltip>
        <TooltipTrigger asChild>
            <span className={cn('inline-flex shrink-0', containerClassName)} tabIndex={disabled ? 0 : undefined}>
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
