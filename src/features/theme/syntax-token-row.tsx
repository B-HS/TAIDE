import type { FC } from 'react'
import { RotateCcw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SyntaxStyle } from '@shared/api/bindings'
import { ColorPicker } from '@features/theme/color-picker'
import { cn } from '@shared/lib/cn'

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
                <button
                    type='button'
                    onClick={() => onChange({ bold: !style.bold })}
                    aria-pressed={style.bold}
                    className={cn(
                        'border-app-border text-app-foreground flex size-6 items-center justify-center rounded-sm border text-xs font-bold',
                        style.bold && 'bg-app-sidebar-item-active',
                    )}>
                    {t('themeEditor.boldAbbreviation')}
                </button>
                <button
                    type='button'
                    onClick={() => onChange({ italic: !style.italic })}
                    aria-pressed={style.italic}
                    className={cn(
                        'border-app-border text-app-foreground flex size-6 items-center justify-center rounded-sm border text-xs italic',
                        style.italic && 'bg-app-sidebar-item-active',
                    )}>
                    {t('themeEditor.italicAbbreviation')}
                </button>
                {changed && (
                    <button
                        type='button'
                        onClick={onReset}
                        aria-label={t('themeEditor.resetToken')}
                        className='text-app-sidebar-icon-default hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'>
                        <RotateCcw className='size-3.5' />
                    </button>
                )}
                <ColorPicker value={style.fg} onChange={(fg) => onChange({ fg })} />
            </div>
        </div>
    )
}
