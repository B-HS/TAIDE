const HEX_COLOR_PATTERN = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/
const TRANSPARENT_KEYWORD = 'transparent'
const SHORT_HEX_LENGTH = 4
const HEX_RADIX = 16
const RGB_CHANNEL_MAX = 255
const HUE_DEGREES = 360
const HUE_SEGMENT = 60
const PERCENT_MAX = 100
export const HEX_ALPHA_LENGTH = 9
export const ALPHA_CHANNEL_MAX = 255
const OPAQUE_HEX_LENGTH = 7

export type Rgb = { r: number; g: number; b: number }
export type Hsv = { h: number; s: number; v: number }

export const isHexColor = (value: string) => HEX_COLOR_PATTERN.test(value.trim())

export const isTransparentKeyword = (value: string) => value.trim().toLowerCase() === TRANSPARENT_KEYWORD

export const isValidThemeColorValue = (value: string) => isHexColor(value) || isTransparentKeyword(value)

export const normalizeHexColor = (value: string) => {
    const trimmed = value.trim()
    if (!isHexColor(trimmed)) return null
    const withHash = trimmed.startsWith('#') ? trimmed : `#${trimmed}`
    return withHash.toLowerCase()
}

const expandShortHex = (hex: string) => (hex.length === SHORT_HEX_LENGTH ? `#${hex[1]}${hex[1]}${hex[2]}${hex[2]}${hex[3]}${hex[3]}` : hex)

export const hexToRgb = (value: string): Rgb | null => {
    const normalized = normalizeHexColor(value)
    if (!normalized) return null
    const expanded = expandShortHex(normalized)
    const r = Number.parseInt(expanded.slice(1, 3), HEX_RADIX)
    const g = Number.parseInt(expanded.slice(3, 5), HEX_RADIX)
    const b = Number.parseInt(expanded.slice(5, 7), HEX_RADIX)
    return { r, g, b }
}

const toHexChannel = (channel: number) =>
    Math.round(Math.min(RGB_CHANNEL_MAX, Math.max(0, channel)))
        .toString(HEX_RADIX)
        .padStart(2, '0')

export const rgbToHex = ({ r, g, b }: Rgb) => `#${toHexChannel(r)}${toHexChannel(g)}${toHexChannel(b)}`

/**
 * Composites a possibly-translucent 8-digit hex color (`#rrggbbaa`) over an opaque background —
 * WCAG relative luminance assumes an opaque color, so measuring a translucent value directly would
 * score a color nothing on screen actually renders. Any other hex form (3- or 6-digit, already
 * opaque) passes through unchanged — an identity operation for colors that carry no alpha channel.
 */
export const compositeOverBackground = (foregroundHex: string, backgroundHex: string) => {
    if (foregroundHex.length !== HEX_ALPHA_LENGTH) return foregroundHex
    const foregroundRgb = hexToRgb(foregroundHex.slice(0, OPAQUE_HEX_LENGTH))
    const backgroundRgb = hexToRgb(backgroundHex)
    if (!foregroundRgb || !backgroundRgb) return foregroundHex.slice(0, OPAQUE_HEX_LENGTH)
    const alpha = Number.parseInt(foregroundHex.slice(OPAQUE_HEX_LENGTH, HEX_ALPHA_LENGTH), HEX_RADIX) / ALPHA_CHANNEL_MAX
    return rgbToHex({
        r: foregroundRgb.r * alpha + backgroundRgb.r * (1 - alpha),
        g: foregroundRgb.g * alpha + backgroundRgb.g * (1 - alpha),
        b: foregroundRgb.b * alpha + backgroundRgb.b * (1 - alpha),
    })
}

export const rgbToHsv = ({ r, g, b }: Rgb): Hsv => {
    const rNorm = r / RGB_CHANNEL_MAX
    const gNorm = g / RGB_CHANNEL_MAX
    const bNorm = b / RGB_CHANNEL_MAX
    const max = Math.max(rNorm, gNorm, bNorm)
    const min = Math.min(rNorm, gNorm, bNorm)
    const delta = max - min

    const hue = () => {
        if (delta === 0) return 0
        if (max === rNorm) return HUE_SEGMENT * (((gNorm - bNorm) / delta) % 6)
        if (max === gNorm) return HUE_SEGMENT * ((bNorm - rNorm) / delta + 2)
        return HUE_SEGMENT * ((rNorm - gNorm) / delta + 4)
    }

    const h = (hue() + HUE_DEGREES) % HUE_DEGREES
    const s = max === 0 ? 0 : delta / max
    const v = max

    return { h, s, v }
}

export const hsvToRgb = ({ h, s, v }: Hsv): Rgb => {
    const c = v * s
    const x = c * (1 - Math.abs(((h / HUE_SEGMENT) % 2) - 1))
    const m = v - c

    const segment = Math.floor(h / HUE_SEGMENT) % 6
    const bySegment: Rgb[] = [
        { r: c, g: x, b: 0 },
        { r: x, g: c, b: 0 },
        { r: 0, g: c, b: x },
        { r: 0, g: x, b: c },
        { r: x, g: 0, b: c },
        { r: c, g: 0, b: x },
    ]
    const picked = bySegment[(segment + 6) % 6]

    return {
        r: (picked.r + m) * RGB_CHANNEL_MAX,
        g: (picked.g + m) * RGB_CHANNEL_MAX,
        b: (picked.b + m) * RGB_CHANNEL_MAX,
    }
}

export const hexToHsv = (value: string): Hsv | null => {
    const rgb = hexToRgb(value)
    return rgb ? rgbToHsv(rgb) : null
}

export const hsvToHex = (hsv: Hsv) => rgbToHex(hsvToRgb(hsv))

export const clampPercent = (value: number) => Math.min(PERCENT_MAX, Math.max(0, value))
