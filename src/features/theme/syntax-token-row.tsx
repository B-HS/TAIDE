import type { FC } from 'react'
import { RotateCcw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SyntaxStyle } from '@shared/api/bindings'
import { ColorPicker } from '@features/theme/color-picker'
import { cn } from '@shared/lib/cn'
import { IconButton } from '@shared/ui/icon-button'

type SyntaxTokenRowProps = {
    label: string
    style: SyntaxStyle
    changed: boolean
    onChange: (patch: Partial<SyntaxStyle>) => void
    onReset: () => void
}

export const SyntaxTokenRow: FC<SyntaxTokenRowProps> = ({ label, style, changed, onChange, onReset }) => {
    const { t } = useTranslation()

    return (
        <div className='flex items-center justify-between gap-2 py-1'>
            <span
                className={cn('truncate text-xs', changed && 'font-medium')}
                style={{ color: style.fg, fontWeight: style.bold ? 700 : undefined, fontStyle: style.italic ? 'italic' : undefined }}>
                {label}
            </span>
            <div className='flex shrink-0 items-center gap-1'>
                <IconButton
                    onClick={() => onChange({ bold: !style.bold })}
                    aria-pressed={style.bold}
                    label={t('themeEditor.boldToggle')}
                    icon={t('themeEditor.boldAbbreviation')}
                    side='bottom'
                    className={cn(
                        'border-app-border text-app-foreground flex size-6 items-center justify-center rounded-sm border text-xs font-bold',
                        style.bold && 'bg-app-sidebar-item-active',
                    )}
                />
                <IconButton
                    onClick={() => onChange({ italic: !style.italic })}
                    aria-pressed={style.italic}
                    label={t('themeEditor.italicToggle')}
                    icon={t('themeEditor.italicAbbreviation')}
                    side='bottom'
                    className={cn(
                        'border-app-border text-app-foreground flex size-6 items-center justify-center rounded-sm border text-xs italic',
                        style.italic && 'bg-app-sidebar-item-active',
                    )}
                />
                {changed && (
                    <IconButton
                        onClick={onReset}
                        label={t('themeEditor.resetToken')}
                        icon={<RotateCcw className='size-3.5' />}
                        side='bottom'
                        className='text-app-sidebar-icon-default hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'
                    />
                )}
                <ColorPicker value={style.fg} onChange={(fg) => onChange({ fg })} />
            </div>
        </div>
    )
}
