import type { FC } from 'react'
import { X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { IconButton } from '@shared/ui/icon-button'

type ShellSlotHeaderProps = {
    label: string
    canClose: boolean
    onClose: () => void
}

/**
 * The thin bar that names one shell slot's project and closes the slot. Rendered only while the
 * window holds more than one slot, so a single-project window looks exactly as it did before shell
 * slots existed; `canClose` still guards the button because the last slot can never be closed
 * (`shell_slot_close` refuses it server-side).
 */
export const ShellSlotHeader: FC<ShellSlotHeaderProps> = ({ label, canClose, onClose }) => {
    const { t } = useTranslation()

    return (
        <div className='border-tab-bar-tab-border bg-app-sidebar-background flex h-6 shrink-0 items-center gap-1 border-b px-2'>
            <span className='text-app-foreground/70 min-w-0 flex-1 truncate text-xs'>{label}</span>
            <IconButton
                label={t('shellSlot.close')}
                icon={<X className='size-3' />}
                disabled={!canClose}
                onClick={onClose}
                side='bottom'
                className='hover:bg-app-sidebar-item-hover flex size-4 shrink-0 items-center justify-center rounded-sm disabled:opacity-40'
            />
        </div>
    )
}
