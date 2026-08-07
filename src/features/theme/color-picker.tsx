import type { FC, FocusEvent, PointerEvent } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { hexToHsv, hsvToHex, isTransparentKeyword, isValidThemeColorValue, normalizeHexColor } from '@shared/lib/color'
import { cn } from '@shared/lib/cn'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'

const SV_SQUARE_SIZE_PX = 176
const HUE_SLIDER_HEIGHT_PX = 14
const FULL_HUE_DEGREES = 360
const DEFAULT_HUE = 0
const DEFAULT_SATURATION = 1
const DEFAULT_VALUE = 1

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
            <PopoverTrigger asChild>
                <button
                    type='button'
                    className='border-app-border flex h-6 items-center gap-1.5 rounded-sm border px-1.5'
                    aria-label={t('themeEditor.pickColor')}>
                    <span
                        className={cn('size-4 shrink-0 rounded-xs border border-app-border', isTransparent && 'bg-panel-input-background')}
                        style={isTransparent ? undefined : { backgroundColor: value }}
                    />
                    <span className='text-app-foreground font-mono text-[11px]'>{isTransparent ? t('themeEditor.transparentLabel') : value}</span>
                </button>
            </PopoverTrigger>
            <PopoverContent className='w-52 space-y-3'>
                <div
                    ref={squareRef}
                    onPointerDown={handleSquarePointerDown}
                    onPointerMove={handleSquarePointerMove}
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
                    onPointerDown={handleHuePointerDown}
                    onPointerMove={handleHuePointerMove}
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
