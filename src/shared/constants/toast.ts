import type { ToasterProps } from 'sonner'

export const TOAST_VERTICAL_POSITIONS = ['top', 'middle', 'bottom'] as const
export const TOAST_HORIZONTAL_POSITIONS = ['left', 'center', 'right'] as const

export type ToastVerticalPosition = (typeof TOAST_VERTICAL_POSITIONS)[number]
export type ToastHorizontalPosition = (typeof TOAST_HORIZONTAL_POSITIONS)[number]

export const DEFAULT_TOAST_POSITION = 'bottom-right'

export const MIDDLE_TOAST_CLASS = 'taide-toaster-middle'

const isVertical = (value: string): value is ToastVerticalPosition => TOAST_VERTICAL_POSITIONS.includes(value as ToastVerticalPosition)

const isHorizontal = (value: string): value is ToastHorizontalPosition => TOAST_HORIZONTAL_POSITIONS.includes(value as ToastHorizontalPosition)

const FALLBACK_POSITION = { vertical: 'bottom', horizontal: 'right' } satisfies {
    vertical: ToastVerticalPosition
    horizontal: ToastHorizontalPosition
}

export const parseToastPosition = (value: string) => {
    const [vertical, horizontal] = value.split('-')
    if (!vertical || !horizontal || !isVertical(vertical) || !isHorizontal(horizontal)) return FALLBACK_POSITION
    return { vertical, horizontal }
}

export const toSonnerPosition = (value: string): NonNullable<ToasterProps['position']> => {
    const { vertical, horizontal } = parseToastPosition(value)
    return vertical === 'middle' ? `top-${horizontal}` : `${vertical}-${horizontal}`
}

export const isMiddleToastPosition = (value: string) => parseToastPosition(value).vertical === 'middle'
