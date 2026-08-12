import type { FC } from 'react'
import { RotateCcw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ColorPicker } from '@features/theme/color-picker'
import { cn } from '@shared/lib/cn'
import { IconButton } from '@shared/ui/icon-button'

type ColorTokenRowProps = {
    label: string
    value: string
    changed: boolean
    onChange: (value: string) => void
    onReset: () => void
}

export const ColorTokenRow: FC<ColorTokenRowProps> = ({ label, value, changed, onChange, onReset }) => {
    const { t } = useTranslation()

    return (
        <div className='flex items-center justify-between gap-2 py-1'>
            <span className={cn('text-app-foreground truncate text-xs', changed && 'font-medium')}>{label}</span>
            <div className='flex shrink-0 items-center gap-1'>
                {changed && (
                    <IconButton
                        onClick={onReset}
                        label={t('themeEditor.resetToken')}
                        icon={<RotateCcw className='size-3.5' />}
                        side='bottom'
                        className='text-app-sidebar-icon-default hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'
                    />
                )}
                <ColorPicker value={value} onChange={onChange} />
            </div>
        </div>
    )
}
