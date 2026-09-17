import type { FC } from 'react'
import { TriangleAlert, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { SHELL_SLOT_FOCUS_IGNORE_ATTRIBUTE } from '@shared/lib/shell-slot'
import { IconButton } from '@shared/ui/icon-button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

type ShellSlotHeaderProps = {
    label: string
    /**
     * Whether this slot's project root is gone from disk (`ProjectRef.root_missing`). A restored
     * project whose drive was unplugged renders an empty-looking tree that is visually identical to
     * an empty folder, and the rail plus this header were the only surfaces showing it with no way
     * to know (d-67 #14) — `Open Recent` and the Welcome list already disable the same entry.
     */
    rootMissing: boolean
    canClose: boolean
    onClose: () => void
}

/**
 * The thin bar that names one shell slot's project and closes the slot. Rendered only while the
 * window holds more than one slot, so a single-project window looks exactly as it did before shell
 * slots existed; `canClose` still guards the button because the last slot can never be closed
 * (`shell_slot_close` refuses it server-side).
 *
 * The close button sits in a {@link SHELL_SLOT_FOCUS_IGNORE_ATTRIBUTE} wrapper rather than carrying
 * the attribute itself, so the disabled state — where `pointer-events-none` puts the tooltip's own
 * wrapper span in the event path instead of the button — is opted out of focus tracking too.
 * `contents` generates no box, so the header's flex layout is unchanged.
 */
export const ShellSlotHeader: FC<ShellSlotHeaderProps> = ({ label, rootMissing, canClose, onClose }) => {
    const { t } = useTranslation()

    return (
        <div className='border-tab-bar-tab-border bg-app-sidebar-background flex h-6 shrink-0 items-center gap-1 border-b px-2'>
            {rootMissing && (
                <Tooltip>
                    <TooltipTrigger asChild>
                        <span role='img' aria-label={t('app.recentProjectRootMissing')} className='flex shrink-0 items-center'>
                            <TriangleAlert className='text-app-sidebar-icon-default size-3' />
                        </span>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('app.recentProjectRootMissing')}</TooltipContent>
                </Tooltip>
            )}
            <span className='text-app-foreground/70 min-w-0 flex-1 truncate text-xs'>{label}</span>
            <span className='contents' {...{ [SHELL_SLOT_FOCUS_IGNORE_ATTRIBUTE]: '' }}>
                <IconButton
                    label={t('shellSlot.close')}
                    icon={<X className='size-3' />}
                    disabled={!canClose}
                    onClick={onClose}
                    side='bottom'
                    className='hover:bg-app-sidebar-item-hover flex size-4 shrink-0 items-center justify-center rounded-sm disabled:opacity-40'
                />
            </span>
        </div>
    )
}
