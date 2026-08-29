import * as React from 'react'
import { Popover as PopoverPrimitive } from 'radix-ui'

import { cn } from '@shared/lib/cn'
import { isImeCompositionKeydown } from '@shared/lib/ime-composition'

function Popover({ ...props }: React.ComponentProps<typeof PopoverPrimitive.Root>) {
    return <PopoverPrimitive.Root data-slot='popover' {...props} />
}

function PopoverTrigger({ ...props }: React.ComponentProps<typeof PopoverPrimitive.Trigger>) {
    return <PopoverPrimitive.Trigger data-slot='popover-trigger' {...props} />
}

function PopoverAnchor({ ...props }: React.ComponentProps<typeof PopoverPrimitive.Anchor>) {
    return <PopoverPrimitive.Anchor data-slot='popover-anchor' {...props} />
}

/** Same IME dismissal guard `DialogContent` documents — popovers host filter inputs too (the font picker's search, the branch switcher's filter), and radix dismisses on the composition-cancelling Escape without it. */
function PopoverContent({
    className,
    align = 'center',
    sideOffset = 4,
    onEscapeKeyDown,
    ...props
}: React.ComponentProps<typeof PopoverPrimitive.Content>) {
    return (
        <PopoverPrimitive.Portal>
            <PopoverPrimitive.Content
                data-slot='popover-content'
                align={align}
                sideOffset={sideOffset}
                onEscapeKeyDown={(event) => {
                    if (isImeCompositionKeydown(event)) {
                        event.preventDefault()
                        return
                    }
                    onEscapeKeyDown?.(event)
                }}
                className={cn(
                    'z-50 w-72 origin-(--radix-popover-content-transform-origin) rounded-md border bg-popover p-4 text-popover-foreground shadow-md outline-none data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95',
                    className,
                )}
                {...props}
            />
        </PopoverPrimitive.Portal>
    )
}

export { Popover, PopoverTrigger, PopoverAnchor, PopoverContent }
