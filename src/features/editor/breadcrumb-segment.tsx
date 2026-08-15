import type { FC } from 'react'
import { ChevronRight } from 'lucide-react'
import { cn } from '@shared/lib/cn'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'

export type BreadcrumbSegmentEntry = {
    key: string
    label: string
    disabled?: boolean
    onSelect?: () => void
}

type BreadcrumbSegmentProps = {
    label: string
    emphasized: boolean
    interactive: boolean
    entries: BreadcrumbSegmentEntry[]
    dropdownAriaLabel: string
    showSeparator: boolean
    onOpenChange?: (open: boolean) => void
}

const SEGMENT_LABEL_CLASS = 'truncate text-xs'
const SEGMENT_EMPHASIZED_CLASS = 'text-editor-foreground font-medium'
const SEGMENT_MUTED_CLASS = 'text-app-sidebar-icon-default'
const SEGMENT_TRIGGER_CLASS =
    'hover:bg-app-sidebar-item-hover hover:text-app-foreground flex items-center gap-0.5 rounded-sm px-1 py-0.5 outline-hidden'

export const BreadcrumbSegment: FC<BreadcrumbSegmentProps> = ({
    label,
    emphasized,
    interactive,
    entries,
    dropdownAriaLabel,
    showSeparator,
    onOpenChange,
}) => (
    <div className='flex shrink-0 items-center gap-0.5'>
        {showSeparator && <ChevronRight className='text-app-sidebar-icon-default size-3.5 shrink-0 opacity-60' />}
        {interactive ? (
            <DropdownMenu onOpenChange={onOpenChange}>
                <DropdownMenuTrigger
                    aria-label={`${dropdownAriaLabel}: ${label}`}
                    className={cn(SEGMENT_TRIGGER_CLASS, emphasized ? SEGMENT_EMPHASIZED_CLASS : SEGMENT_MUTED_CLASS)}>
                    <span className={SEGMENT_LABEL_CLASS}>{label}</span>
                </DropdownMenuTrigger>
                <DropdownMenuContent align='start'>
                    {entries.map((entry) => (
                        <DropdownMenuItem key={entry.key} disabled={entry.disabled} onSelect={entry.onSelect}>
                            {entry.label}
                        </DropdownMenuItem>
                    ))}
                </DropdownMenuContent>
            </DropdownMenu>
        ) : (
            <span className={cn(SEGMENT_LABEL_CLASS, 'px-1 py-0.5', emphasized ? SEGMENT_EMPHASIZED_CLASS : SEGMENT_MUTED_CLASS)}>{label}</span>
        )}
    </div>
)
