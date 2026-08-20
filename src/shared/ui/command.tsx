'use client'

import * as React from 'react'
import { Command as CommandPrimitive } from 'cmdk'
import { SearchIcon } from 'lucide-react'

import { cn } from '@shared/lib/cn'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@shared/ui/dialog'

function Command({ className, ...props }: React.ComponentProps<typeof CommandPrimitive>) {
    return (
        <CommandPrimitive
            data-slot='command'
            className={cn('flex h-full w-full flex-col overflow-hidden rounded-md bg-popover text-popover-foreground', className)}
            {...props}
        />
    )
}

function CommandDialog({
    title = 'Command Palette',
    description = 'Search for a command to run...',
    children,
    className,
    showCloseButton = true,
    ...props
}: React.ComponentProps<typeof Dialog> & {
    title?: string
    description?: string
    className?: string
    showCloseButton?: boolean
}) {
    return (
        <Dialog {...props}>
            <DialogHeader className='sr-only'>
                <DialogTitle>{title}</DialogTitle>
                <DialogDescription>{description}</DialogDescription>
            </DialogHeader>
            <DialogContent className={cn('overflow-hidden p-0', className)} showCloseButton={showCloseButton}>
                <Command className='**:data-[slot=command-input-wrapper]:h-12 [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground [&_[cmdk-group]]:px-2 [&_[cmdk-group]:not([hidden])_~[cmdk-group]]:pt-0 [&_[cmdk-input-wrapper]_svg]:h-5 [&_[cmdk-input-wrapper]_svg]:w-5 [&_[cmdk-input]]:h-12 [&_[cmdk-item]]:px-2 [&_[cmdk-item]]:py-3 [&_[cmdk-item]_svg]:h-5 [&_[cmdk-item]_svg]:w-5'>
                    {children}
                </Command>
            </DialogContent>
        </Dialog>
    )
}

function CommandInput({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Input>) {
    return (
        <div data-slot='command-input-wrapper' className='flex h-9 items-center gap-2 border-b px-3'>
            <SearchIcon className='size-4 shrink-0 opacity-50' />
            <CommandPrimitive.Input
                data-slot='command-input'
                className={cn(
                    'flex h-10 w-full rounded-md bg-transparent py-3 text-sm outline-hidden placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50',
                    className,
                )}
                {...props}
            />
        </div>
    )
}

function CommandList({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.List>) {
    return (
        <CommandPrimitive.List
            data-slot='command-list'
            className={cn('scrollbar-hidden max-h-[300px] scroll-py-1 overflow-x-hidden overflow-y-auto', className)}
            {...props}
        />
    )
}

function CommandEmpty({ ...props }: React.ComponentProps<typeof CommandPrimitive.Empty>) {
    return <CommandPrimitive.Empty data-slot='command-empty' className='py-6 text-center text-sm' {...props} />
}

function CommandGroup({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Group>) {
    return (
        <CommandPrimitive.Group
            data-slot='command-group'
            className={cn(
                'overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground',
                className,
            )}
            {...props}
        />
    )
}

function CommandSeparator({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Separator>) {
    return <CommandPrimitive.Separator data-slot='command-separator' className={cn('-mx-1 h-px bg-border', className)} {...props} />
}

/**
 * Selected-row styling deliberately diverges from the shadcn default (`bg-accent`, which resolves to
 * the same swatch as list hover and renders near-invisibly in several bundled themes —
 * `docs/acknowledge/2026-08-20-palette-ux-contract.md` §1.1). `bg-list-active-background` is the
 * guarded token (`shared/lib/theme-convert/mapping-tables.ts`'s `derived()` + `isUsableListBackground`
 * pattern, `docs/acknowledge/2026-08-20-theme-list-colors-contract.md`) so it can never collapse onto
 * the row background, and `ring-app-accent` adds a second, non-background-dependent cue — swept across
 * all 36 bundled themes at >=1.5:1 against `panel.background`, versus `app.focusBorder` which drops
 * below that in 15/36 (`docs/acknowledge/2026-08-20-palette-ux-contract.md` §4). Both tokens apply here
 * (not per call site) because every consumer of `CommandItem` (7 files, all `grep -rl CommandItem src`)
 * shares the same defect surface.
 */
function CommandItem({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Item>) {
    return (
        <CommandPrimitive.Item
            data-slot='command-item'
            className={cn(
                "relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 data-[selected=true]:bg-list-active-background data-[selected=true]:text-accent-foreground data-[selected=true]:ring-1 data-[selected=true]:ring-inset data-[selected=true]:ring-app-accent [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground",
                className,
            )}
            {...props}
        />
    )
}

function CommandShortcut({ className, ...props }: React.ComponentProps<'span'>) {
    return <span data-slot='command-shortcut' className={cn('ml-auto text-xs tracking-widest text-muted-foreground', className)} {...props} />
}

export { Command, CommandDialog, CommandInput, CommandList, CommandEmpty, CommandGroup, CommandItem, CommandShortcut, CommandSeparator }
