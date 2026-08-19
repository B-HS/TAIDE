import type { FC, FocusEvent, KeyboardEvent, PointerEvent } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { hexToHsv, hsvToHex, isTransparentKeyword, isValidThemeColorValue, normalizeHexColor } from '@shared/lib/color'
import { cn } from '@shared/lib/cn'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const SV_SQUARE_SIZE_PX = 176
const HUE_SLIDER_HEIGHT_PX = 14
const FULL_HUE_DEGREES = 360
const DEFAULT_HUE = 0
const DEFAULT_SATURATION = 1
const DEFAULT_VALUE = 1
const SV_VALUE_PERCENT_MAX = 100
const SV_KEYBOARD_STEP = 0.05
const HUE_KEYBOARD_STEP_DEGREES = 1

const SV_SQUARE_ARROW_DELTA: Record<string, { ds: number; dv: number }> = {
    ArrowRight: { ds: SV_KEYBOARD_STEP, dv: 0 },
    ArrowLeft: { ds: -SV_KEYBOARD_STEP, dv: 0 },
    ArrowUp: { ds: 0, dv: SV_KEYBOARD_STEP },
    ArrowDown: { ds: 0, dv: -SV_KEYBOARD_STEP },
}

const HUE_ARROW_DELTA_DEGREES: Record<string, number> = {
    ArrowRight: HUE_KEYBOARD_STEP_DEGREES,
    ArrowUp: HUE_KEYBOARD_STEP_DEGREES,
    ArrowLeft: -HUE_KEYBOARD_STEP_DEGREES,
    ArrowDown: -HUE_KEYBOARD_STEP_DEGREES,
}

type ColorPickerProps = {
    value: string
    onChange: (value: string) => void
}

export const ColorPicker: FC<ColorPickerProps> = ({ value, onChange }) => {
    const { t } = useTranslation()
    const squareRef = useRef<HTMLDivElement>(null)
    const hueRef = useRef<HTMLDivElement>(null)
    const [hexError, setHexError] = useState(false)

    const isTransparent = isTransparentKeyword(value)
    const hsv = !isTransparent ? hexToHsv(value) : null
    const activeHue = hsv?.h ?? DEFAULT_HUE
    const activeSaturation = hsv?.s ?? DEFAULT_SATURATION
    const activeValue = hsv?.v ?? DEFAULT_VALUE
    const hueSwatchHex = hsvToHex({ h: activeHue, s: 1, v: 1 })

    const updateFromSquare = (event: { clientX: number; clientY: number }) => {
        const rect = squareRef.current?.getBoundingClientRect()
        if (!rect) return
        const s = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width))
        const v = Math.min(1, Math.max(0, 1 - (event.clientY - rect.top) / rect.height))
        onChange(hsvToHex({ h: activeHue, s, v }))
    }

    const updateFromHue = (event: { clientX: number }) => {
        const rect = hueRef.current?.getBoundingClientRect()
        if (!rect) return
        const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width))
        const h = ratio * FULL_HUE_DEGREES
        onChange(hsvToHex({ h, s: activeSaturation, v: activeValue }))
    }

    const handleSquarePointerDown = (event: PointerEvent<HTMLDivElement>) => {
        event.currentTarget.setPointerCapture(event.pointerId)
        updateFromSquare(event)
    }

    const handleSquarePointerMove = (event: PointerEvent<HTMLDivElement>) => {
        if (event.buttons !== 1) return
        updateFromSquare(event)
    }

    const handleHuePointerDown = (event: PointerEvent<HTMLDivElement>) => {
        event.currentTarget.setPointerCapture(event.pointerId)
        updateFromHue(event)
    }

    const handleHuePointerMove = (event: PointerEvent<HTMLDivElement>) => {
        if (event.buttons !== 1) return
        updateFromHue(event)
    }

    const handleSquareKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        const delta = SV_SQUARE_ARROW_DELTA[event.key]
        if (!delta) return
        event.preventDefault()
        const nextSaturation = Math.min(1, Math.max(0, activeSaturation + delta.ds))
        const nextValue = Math.min(1, Math.max(0, activeValue + delta.dv))
        onChange(hsvToHex({ h: activeHue, s: nextSaturation, v: nextValue }))
    }

    const handleHueKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        const delta = HUE_ARROW_DELTA_DEGREES[event.key]
        if (delta === undefined) return
        event.preventDefault()
        const nextHue = Math.min(FULL_HUE_DEGREES, Math.max(0, activeHue + delta))
        onChange(hsvToHex({ h: nextHue, s: activeSaturation, v: activeValue }))
    }

    const handleHexBlur = (event: FocusEvent<HTMLInputElement>) => {
        const raw = event.currentTarget.value.trim()
        if (isTransparentKeyword(raw)) {
            setHexError(false)
            onChange('transparent')
            return
        }
        const normalized = normalizeHexColor(raw)
        if (!normalized) {
            setHexError(true)
            return
        }
        setHexError(false)
        onChange(normalized)
    }

    return (
        <Popover>
            <Tooltip>
                <TooltipTrigger asChild>
                    <PopoverTrigger asChild>
                        <button
                            type='button'
                            className='border-app-border flex h-6 items-center gap-1.5 rounded-sm border px-1.5'
                            aria-label={t('themeEditor.pickColor')}>
                            <span
                                className={cn('size-4 shrink-0 rounded-xs border border-app-border', isTransparent && 'bg-panel-input-background')}
                                style={isTransparent ? undefined : { backgroundColor: value }}
                            />
                            <span className='text-app-foreground font-mono text-[11px]'>
                                {isTransparent ? t('themeEditor.transparentLabel') : value}
                            </span>
                        </button>
                    </PopoverTrigger>
                </TooltipTrigger>
                <TooltipContent side='bottom'>{t('themeEditor.pickColor')}</TooltipContent>
            </Tooltip>
            <PopoverContent className='w-52 space-y-3'>
                <div
                    ref={squareRef}
                    role='slider'
                    tabIndex={0}
                    aria-label={t('themeEditor.saturationValueSliderLabel')}
                    aria-valuemin={0}
                    aria-valuemax={SV_VALUE_PERCENT_MAX}
                    aria-valuenow={Math.round(activeValue * SV_VALUE_PERCENT_MAX)}
                    onPointerDown={handleSquarePointerDown}
                    onPointerMove={handleSquarePointerMove}
                    onKeyDown={handleSquareKeyDown}
                    className='relative touch-none rounded-sm'
                    style={{
                        height: SV_SQUARE_SIZE_PX,
                        backgroundColor: hueSwatchHex,
                        backgroundImage: 'linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, transparent)',
                    }}>
                    <span
                        className='pointer-events-none absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow'
                        style={{ left: `${activeSaturation * 100}%`, top: `${(1 - activeValue) * 100}%` }}
                    />
                </div>
                <div
                    ref={hueRef}
                    role='slider'
                    tabIndex={0}
                    aria-label={t('themeEditor.hueSliderLabel')}
                    aria-orientation='horizontal'
                    aria-valuemin={0}
                    aria-valuemax={FULL_HUE_DEGREES}
                    aria-valuenow={Math.round(activeHue)}
                    onPointerDown={handleHuePointerDown}
                    onPointerMove={handleHuePointerMove}
                    onKeyDown={handleHueKeyDown}
                    className='relative touch-none rounded-full'
                    style={{
                        height: HUE_SLIDER_HEIGHT_PX,
                        backgroundImage: 'linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)',
                    }}>
                    <span
                        className='pointer-events-none absolute top-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow'
                        style={{ left: `${(activeHue / FULL_HUE_DEGREES) * 100}%` }}
                    />
                </div>
                <label className='flex flex-col gap-1'>
                    <span className='text-app-sidebar-icon-default text-[11px]'>{t('themeEditor.colorValuePlaceholder')}</span>
                    <input
                        key={value}
                        type='text'
                        defaultValue={value}
                        onBlur={handleHexBlur}
                        placeholder='#rrggbb'
                        className={cn(
                            'bg-panel-input-background border-panel-input-border text-app-foreground rounded-sm border px-2 py-1 font-mono text-xs',
                            hexError && 'border-status-error',
                        )}
                    />
                    {hexError && <span className='text-status-error text-[11px]'>{t('themeEditor.invalidColor')}</span>}
                </label>
                {!isValidThemeColorValue(value) && <span className='text-status-error text-[11px]'>{t('themeEditor.invalidColor')}</span>}
            </PopoverContent>
        </Popover>
    )
}
