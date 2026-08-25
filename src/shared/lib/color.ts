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

/**
 * CIE 1976 L*a*b* (D65, 2° standard observer) conversion constants for {@link deltaE76} below. The
 * linearization threshold/divisor/gamma triple is the sRGB standard (IEC 61966-2-1) — note this is
 * the *color-science* breakpoint (0.04045 in gamma-encoded space), not the WCAG relative-luminance
 * breakpoint (0.03928) that `theme-convert/contrast.ts` uses for a different purpose (contrast
 * ratio, not perceptual distance). The 3x3 matrix is the standard sRGB-to-XYZ (D65) transform and
 * the epsilon/kappa pair is the CIE's own piecewise constants for the f(t) helper in the L*a*b*
 * formula (both from CIE 15:2004 / Bruce Lindbloom's reference derivation).
 */
const CIE_SRGB_LINEAR_THRESHOLD = 0.04045
const CIE_SRGB_LINEAR_DIVISOR = 12.92
const CIE_SRGB_GAMMA_OFFSET = 0.055
const CIE_SRGB_GAMMA_DIVISOR = 1.055
const CIE_SRGB_GAMMA_EXPONENT = 2.4
const CIE_XYZ_MATRIX = {
    x: { r: 0.4124564, g: 0.3575761, b: 0.1804375 },
    y: { r: 0.2126729, g: 0.7151522, b: 0.072175 },
    z: { r: 0.0193339, g: 0.119192, b: 0.9503041 },
}
const CIE_D65_WHITE_POINT = { x: 0.95047, y: 1.0, z: 1.08883 }
const CIE_LAB_EPSILON = 216 / 24389
const CIE_LAB_KAPPA = 24389 / 27
const CIE_LAB_L_SCALE = 116
const CIE_LAB_L_OFFSET = 16
const CIE_LAB_A_SCALE = 500
const CIE_LAB_B_SCALE = 200

export type Rgb = { r: number; g: number; b: number }
export type Hsv = { h: number; s: number; v: number }
export type Lab = { l: number; a: number; b: number }

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

const srgbChannelToCieLinear = (channel: number) => {
    const normalized = channel / RGB_CHANNEL_MAX
    return normalized <= CIE_SRGB_LINEAR_THRESHOLD
        ? normalized / CIE_SRGB_LINEAR_DIVISOR
        : ((normalized + CIE_SRGB_GAMMA_OFFSET) / CIE_SRGB_GAMMA_DIVISOR) ** CIE_SRGB_GAMMA_EXPONENT
}

const cieLabF = (t: number) => (t > CIE_LAB_EPSILON ? Math.cbrt(t) : (CIE_LAB_KAPPA * t + CIE_LAB_L_OFFSET) / CIE_LAB_L_SCALE)

const hexToLab = (value: string): Lab | null => {
    const rgb = hexToRgb(value)
    if (!rgb) return null
    const rLinear = srgbChannelToCieLinear(rgb.r)
    const gLinear = srgbChannelToCieLinear(rgb.g)
    const bLinear = srgbChannelToCieLinear(rgb.b)
    const x = rLinear * CIE_XYZ_MATRIX.x.r + gLinear * CIE_XYZ_MATRIX.x.g + bLinear * CIE_XYZ_MATRIX.x.b
    const y = rLinear * CIE_XYZ_MATRIX.y.r + gLinear * CIE_XYZ_MATRIX.y.g + bLinear * CIE_XYZ_MATRIX.y.b
    const z = rLinear * CIE_XYZ_MATRIX.z.r + gLinear * CIE_XYZ_MATRIX.z.g + bLinear * CIE_XYZ_MATRIX.z.b
    const fx = cieLabF(x / CIE_D65_WHITE_POINT.x)
    const fy = cieLabF(y / CIE_D65_WHITE_POINT.y)
    const fz = cieLabF(z / CIE_D65_WHITE_POINT.z)
    return {
        l: CIE_LAB_L_SCALE * fy - CIE_LAB_L_OFFSET,
        a: CIE_LAB_A_SCALE * (fx - fy),
        b: CIE_LAB_B_SCALE * (fy - fz),
    }
}

/**
 * Perceptual color distance (CIE76 ΔE*ab) between two hex colors — the straight-line Euclidean
 * distance in L*a*b* space. Unlike WCAG contrast ratio (`theme-convert/contrast.ts`), this measures
 * how visually *distinguishable* two colors are regardless of which is lighter/darker, so it does
 * not produce false negatives for a pair like a saturated yellow on a similarly-luminant gray
 * (WCAG contrast ~1:1, clearly distinct to the eye — see `docs/acknowledge/2026-08-24-d33-restructure-carryover-contract.md`
 * §"임무 C" for the github-dark case: `#ffd33d` vs `#d1d5da` scores ΔE≈77.7 despite ~1.03:1 contrast).
 * CIE76 (rather than the perceptually-uniform-corrected CIE94/CIEDE2000) is sufficient here: applied
 * to the 36 bundled themes' `panel.matchHighlight` vs `app.foreground` pairs, it produces three
 * exact-duplicate hits at 0.0 and then a clean jump to 5.4+ with no borderline values in between —
 * more sophisticated formulas would only refine values that are already unambiguous for this use.
 * The alpha channel (8-digit `#rrggbbaa` input) is ignored — `hexToLab`/`hexToRgb` only ever read
 * the first 6 hex digits — because this measures perceptual distance between the colors
 * themselves, not what ends up on screen (unlike `compositeOverBackground`, which callers use when
 * the rendered-over-a-background color is what matters, e.g. contrast ratio). Returns `null` when
 * either input is not a parseable hex color.
 */
export const deltaE76 = (hexA: string, hexB: string): number | null => {
    const labA = hexToLab(hexA)
    const labB = hexToLab(hexB)
    if (!labA || !labB) return null
    return Math.sqrt((labA.l - labB.l) ** 2 + (labA.a - labB.a) ** 2 + (labA.b - labB.b) ** 2)
}
