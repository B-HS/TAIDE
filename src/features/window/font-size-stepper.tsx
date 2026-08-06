import type { FC, ReactNode } from 'react'
import { Minus, Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

type FontSizeStepperProps = {
    label: string
    icon: ReactNode
    value: number
    onDecrease: () => void
    onIncrease: () => void
    onReset: () => void
}

const STEP_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-4 items-center justify-center rounded-sm'

export const FontSizeStepper: FC<FontSizeStepperProps> = ({ label, icon, value, onDecrease, onIncrease, onReset }) => {
    const { t } = useTranslation()

    return (
        <div className='flex shrink-0 items-center gap-0.5'>
            <Tooltip>
                <TooltipTrigger asChild>
                    <span className='text-app-sidebar-icon-default flex size-3.5 items-center justify-center'>{icon}</span>
                </TooltipTrigger>
                <TooltipContent side='top'>{label}</TooltipContent>
            </Tooltip>
            <Tooltip>
                <TooltipTrigger asChild>
                    <button type='button' aria-label={t('window.decreaseFontSize', { label })} onClick={onDecrease} className={STEP_BUTTON_CLASS}>
                        <Minus className='size-3' />
                    </button>
                </TooltipTrigger>
                <TooltipContent side='top'>{t('window.decreaseFontSize', { label })}</TooltipContent>
            </Tooltip>
            <Tooltip>
                <TooltipTrigger asChild>
                    <button
                        type='button'
                        aria-label={t('window.resetFontSize', { label })}
                        onClick={onReset}
                        className='text-app-foreground w-6 text-center tabular-nums'>
                        {value}
                    </button>
                </TooltipTrigger>
                <TooltipContent side='top'>{t('window.resetFontSize', { label })}</TooltipContent>
            </Tooltip>
            <Tooltip>
                <TooltipTrigger asChild>
                    <button type='button' aria-label={t('window.increaseFontSize', { label })} onClick={onIncrease} className={STEP_BUTTON_CLASS}>
                        <Plus className='size-3' />
                    </button>
                </TooltipTrigger>
                <TooltipContent side='top'>{t('window.increaseFontSize', { label })}</TooltipContent>
            </Tooltip>
        </div>
    )
}
